use std::error::Error;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::{mem, thread};
use std::convert::identity;
use std::future::Future;
use std::ops::DerefMut;
use std::pin::{Pin, pin};
use std::sync::mpsc::{channel, SendError};
use std::task::{Context, Poll};
use async_channel::Receiver;
use futures::{FutureExt, Stream, TryFutureExt};
use gtk::gio::spawn_blocking;
use gtk::glib;
use log::error;
use pin_project::{pin_project, pinned_drop};

use error::Result;

pub mod libusb;
pub mod error;
pub mod foxdelta;
pub mod dummy;
mod commands;

pub trait SWRAnalyzer {
    fn version(&mut self) -> Result<String>;
    fn set_led_blink(&mut self, state: LedState) -> Result<()>;
    fn start_oneshot(&mut self,
                     noise_filter: i32,
                     start_frequency: i32,
                     step_frequency: i32,
                     max_step_count: i32,
                     step_millis: i32,
                     f: &mut dyn FnMut(i32, i32, i32) -> bool) -> Result<()>;
}

#[derive(Debug)]
pub enum LedState {
    Off,
    Blink,
}

enum AnalyzerState<D> {
    Available(D),
    Busy,
    Err,
}

pub struct AsyncSWRAnalyzer<D> {
    analyzer: AnalyzerState<D>,
}

enum DataSample<T> {
    Sample(i32, i32, i32),
    Done(T),
}

impl<D: DerefMut + Send + 'static> AsyncSWRAnalyzer<D> where D::Target: SWRAnalyzer {
    pub fn new(analyzer: D) -> Self {
        Self {
            analyzer: AnalyzerState::Available(analyzer)
        }
    }

    async fn run_blocking<T: Send + 'static, F: Send + 'static + FnOnce(&mut D) -> T>(&mut self, f: F) -> Result<T> {
        let mut analyzer = match mem::replace(&mut self.analyzer, AnalyzerState::Busy) {
            AnalyzerState::Available(analyzer) => analyzer,
            AnalyzerState::Busy => {
                return Err(error::Error::Busy);
            }
            AnalyzerState::Err => {
                self.analyzer = AnalyzerState::Err;
                return Err(error::Error::PreviousError);
            }
        };
        match spawn_blocking(move || {
            let result = f(&mut analyzer);
            (analyzer, result)
        }).await {
            Ok((analyzer, result)) => {
                self.analyzer = AnalyzerState::Available(analyzer);
                Ok(result)
            }
            Err(e) => {
                self.analyzer = AnalyzerState::Err;
                Err(error::Error::ThreadError(e))
            }
        }
    }

    pub async fn version(&mut self) -> Result<String> {
        self.run_blocking(|this| this.version()).await.and_then(identity)
    }

    pub async fn set_led_blink(&mut self, state: LedState) -> Result<()> {
        self.run_blocking(|this| this.set_led_blink(state)).await.and_then(identity)
    }

    pub fn start_oneshot<'a>(&'a mut self,
                             noise_filter: i32,
                             start_frequency: i32,
                             step_frequency: i32,
                             max_step_count: i32,
                             step_millis: i32) -> ScanStream<impl Future<Output=Result<()>> + 'a> {
        let (send, recv) = async_channel::unbounded();

        let task = self.run_blocking(move |this| {
            this.start_oneshot(noise_filter,
                               start_frequency,
                               step_frequency,
                               max_step_count,
                               step_millis, &mut move |i, freq, sample| {
                    match send.send_blocking((i, freq, sample)) {
                        Ok(_) => true,
                        Err(_) => false,
                    }
                })
        }).and_then(|x| async { x });

        ScanStream {
            task_done: false,
            task,
            recv,
        }
    }
}

#[pin_project(PinnedDrop)]
pub struct ScanStream<T: Future<Output=Result<()>>> {
    task_done: bool,
    #[pin] task: T,
    #[pin] recv: Receiver<(i32, i32, i32)>,
}

impl<T: Future<Output=Result<()>>> Stream for ScanStream<T> {
    type Item = Result<(i32, i32, i32)>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let mut pinned = self.project();
        if !*pinned.task_done {
            match pinned.task.poll(cx) {
                Poll::Ready(Err(e)) => {
                    *pinned.task_done = true;
                    return Poll::Ready(Some(Err(e)));
                },
                Poll::Ready(Ok(())) => {
                    *pinned.task_done = true;
                }
                _ => {}
            }
        }
        pinned.recv.as_mut().poll_next(cx).map(|x| x.map(|x| Ok(x)))
    }
}

#[pinned_drop]
impl<T: Future<Output=Result<()>>> PinnedDrop for ScanStream<T> {
    fn drop(self: Pin<&mut Self>) {
        if !self.task_done {
            panic!("Dropped without finalizing")
        }
    }
}

impl<T: Future<Output=Result<()>>> ScanStream<T> {
    pub async fn cancel(self: Pin<&mut Self>) -> Result<()> {
        self.recv.close();
        if !self.task_done {
            let this = self.project();
            
            let result = this.task.await;
            *this.task_done = true;
            result
        } else {
            Ok(())
        }
    }
}
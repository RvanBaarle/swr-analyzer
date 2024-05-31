use std::convert::identity;
use std::fmt::{Debug, Formatter};
use std::mem;
use std::ops::ControlFlow::{Break, Continue};
use std::ops::DerefMut;
use std::pin::{Pin, pin};
use std::task::{Context, Poll};

use async_channel::Receiver;
use futures::{FutureExt, Stream, TryFutureExt};
use futures::future::FusedFuture;
use gtk4::gio::spawn_blocking;
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
    fn start_sweep(&mut self,
                   continuous: bool,
                   noise_filter: i32,
                   start_frequency: i32,
                   step_frequency: i32,
                   max_step_count: i32,
                   step_millis: i32,
                   f: &mut dyn FnMut(i32, i32, i32) -> std::ops::ControlFlow<()>) -> Result<()>;
    fn start_continuous(&mut self,
                        noise_filter: i32,
                        start_frequency: i32,
                        step_frequency: i32,
                        max_step_count: i32,
                        step_millis: i32,
                        f: &mut dyn FnMut(i32, i32, i32) -> std::ops::ControlFlow<()>) -> Result<()> {
        self.start_sweep(true, noise_filter, start_frequency, step_frequency, max_step_count, step_millis, f)
    }
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
                return Err(error::Error::Previous);
            }
        };
        match tokio::task::spawn_blocking(move || {
            let result = f(&mut analyzer);
            (analyzer, result)
        }).await {
            Ok((analyzer, result)) => {
                self.analyzer = AnalyzerState::Available(analyzer);
                Ok(result)
            }
            Err(e) => {
                self.analyzer = AnalyzerState::Err;
                Err(error::Error::Thread(e))
            }
        }
    }

    pub async fn version(&mut self) -> Result<String> {
        self.run_blocking(|this| this.version()).await.and_then(identity)
    }

    pub async fn set_led_blink(&mut self, state: LedState) -> Result<()> {
        self.run_blocking(|this| this.set_led_blink(state)).await.and_then(identity)
    }

    pub fn start_oneshot(&mut self,
                         noise_filter: i32,
                         start_frequency: i32,
                         step_frequency: i32,
                         max_step_count: i32,
                         step_millis: i32) -> ScanStream {
        let (send, recv) = async_channel::unbounded();

        let task = self.run_blocking(move |this| {
            this.start_sweep(
                false, noise_filter,
                start_frequency,
                step_frequency,
                max_step_count,
                step_millis, &mut move |i, freq, sample| {
                    send.send_blocking((i, freq, sample))
                        .map_or_else(|_| Break(()), |_| Continue(()))
                })
        }).and_then(|x| async { x }).fuse();

        ScanStream {
            task: Box::pin(task),
            recv,
        }
    }

    pub fn start_continuous(&mut self,
                            noise_filter: i32,
                            start_frequency: i32,
                            step_frequency: i32,
                            max_step_count: i32,
                            step_millis: i32) -> ScanStream {
        let (send, recv) = async_channel::unbounded();

        let task = self.run_blocking(move |this| {
            this.start_continuous(noise_filter,
                                  start_frequency,
                                  step_frequency,
                                  max_step_count,
                                  step_millis, &mut move |i, freq, sample| {
                    send.send_blocking((i, freq, sample))
                        .map_or_else(|_| Break(()), |_| Continue(()))
                })
        }).and_then(|x| async { x }).fuse();

        ScanStream {
            task: Box::pin(task),
            recv,
        }
    }
}

#[pin_project(PinnedDrop)]
pub struct ScanStream<'a> {
    task: Pin<Box<dyn FusedFuture<Output=Result<()>> + Send + 'a>>,
    #[pin] recv: Receiver<(i32, i32, i32)>,
}

impl Stream for ScanStream<'_> {
    type Item = Result<(i32, i32, i32)>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let mut pinned = self.project();
        if let Poll::Ready(Err(e)) = pinned.task.poll_unpin(cx) {
            return Poll::Ready(Some(Err(e)));
        }
        pinned.recv.as_mut().poll_next(cx).map(|x| x.map(Ok))
    }
}

#[pinned_drop]
impl PinnedDrop for ScanStream<'_> {
    fn drop(self: Pin<&mut Self>) {
        if !self.task.is_terminated() {
            panic!("Dropped without finalizing")
        }
    }
}

impl ScanStream<'_> {
    pub async fn cancel(self: Pin<&mut Self>) -> Result<()> {
        self.recv.close();
        if !self.task.is_terminated() {
            let this = self.project();
            this.task.await
        } else {
            Ok(())
        }
    }
}
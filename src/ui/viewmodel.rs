use std::sync::Mutex;

pub struct ViewModel<T: Send> {
    update: Box<dyn FnMut() + Send>,
    data: Mutex<T>,
}
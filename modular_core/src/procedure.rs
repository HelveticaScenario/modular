use crossbeam_channel::bounded;

use crossbeam_channel::Receiver;

use crossbeam_channel::Sender;

pub struct Procedure<T, R> {
    pub(crate) tx: Sender<Box<dyn FnOnce(T) -> R + Send>>,
    pub(crate) rx: Receiver<R>,
}

impl<T, R> Procedure<T, R> {
    pub fn call(&self, cb: Box<dyn FnOnce(T) -> R + Send>) -> R {
        self.tx.send(cb).unwrap();
        self.rx.recv().unwrap()
    }
}

pub struct ProcedureHandler<T, R> {
    pub(crate) tx: Sender<R>,
    pub rx: Receiver<Box<dyn FnOnce(T) -> R + Send>>,
}

impl<T, R> ProcedureHandler<T, R> {
    pub fn handle(&self, arg: T, cb: Box<dyn FnOnce(T) -> R + Send>) {
        self.tx.send(cb(arg)).unwrap()
    }
}

pub fn new_procedure<T, R>() -> (Procedure<T, R>, ProcedureHandler<T, R>) {
    let (fn_tx, fn_rx) = bounded::<Box<dyn FnOnce(T) -> R + Send>>(1);
    let (response_tx, response_rx) = bounded::<R>(1);
    (
        Procedure {
            tx: fn_tx,
            rx: response_rx,
        },
        ProcedureHandler {
            tx: response_tx,
            rx: fn_rx,
        },
    )
}

use {
    anyhow::Error,
    flume::{Receiver, RecvError, Sender},
    once_cell::sync::OnceCell,
    slotmap::{DefaultKey, SlotMap},
    std::thread::JoinHandle,
};

#[macro_export]
macro_rules! report {
    ($e:expr) => {
        $crate::report_error(anyhow::anyhow!($e));
    };
}

const INIT_MSG: &'static str = "init() should be called once";

pub enum Message {
    Error(Error, Sender<DefaultKey>),
    Update(DefaultKey, String),
    ForEach(fn(&MyError)),
    Quit,
}

impl std::fmt::Debug for Message {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Message::Error(err, _) => write!(f, "Error({err:?})"),
            Message::Update(_, s) => write!(f, "Update({s:?})"),
            Message::ForEach(_) => write!(f, "ForEach(...)"),
            Message::Quit => write!(f, "Quit"),
        }
    }
}

unsafe impl Sync for Message {}
unsafe impl Send for Message {}

static MSG_TX: OnceCell<Sender<Message>> = OnceCell::new();

#[derive(Debug)]
pub struct MyError {
    error: Error,
    extra: Option<String>,
}

impl MyError {
    pub fn error(&self) -> &Error {
        &self.error
    }

    pub fn extra(&self) -> Option<&String> {
        self.extra.as_ref()
    }
}

pub struct ErrorThread {
    handle: Option<JoinHandle<SlotMap<DefaultKey, MyError>>>,
}

impl ErrorThread {
    pub fn done(mut self) -> SlotMap<DefaultKey, MyError> {
        println!("called done");
        let tx = MSG_TX.get().expect(INIT_MSG).clone();
        tx.send(Message::Quit).unwrap();
        self.handle.take().unwrap().join().unwrap()
    }
}

impl Drop for ErrorThread {
    fn drop(&mut self) {
        println!("dropped");
        let tx = MSG_TX.get().expect(INIT_MSG).clone();
        let _x = tx.send(Message::Quit);
    }
}

fn handle_messages(message_rx: Receiver<Message>) -> SlotMap<DefaultKey, MyError> {
    let mut errors = SlotMap::new();

    loop {
        let message = message_rx.recv();
        println!("get message {message:?}");
        match message {
            Ok(Message::Error(error, sender)) => {
                let key = errors.insert(MyError { error, extra: None });
                sender.send(key).unwrap();
            }

            Ok(Message::Update(key, extra)) => {
                if let Some(error) = errors.get_mut(key) {
                    error.extra = Some(extra);
                }
            }

            Ok(Message::ForEach(f)) => {
                for (_, error) in errors.iter() {
                    f(error);
                }
            }

            Ok(Message::Quit) => {
                break;
            }

            Err(RecvError::Disconnected) => {
                break;
            }
        }
    }

    println!("exiting");

    errors
}

pub fn init() -> ErrorThread {
    let (message_tx, message_rx) = flume::unbounded();
    MSG_TX.set(message_tx).expect(INIT_MSG);

    let handle = std::thread::spawn(|| handle_messages(message_rx));

    ErrorThread {
        handle: Some(handle),
    }
}

pub fn report_error(error: Error) -> DefaultKey {
    let msg_tx = MSG_TX.get().expect(INIT_MSG).clone();
    let (key_tx, key_rx) = flume::bounded(1);
    msg_tx.send(Message::Error(error, key_tx)).unwrap();
    key_rx.recv().unwrap()
}

pub fn update_error(key: DefaultKey, extra: String) {
    let msg_tx = MSG_TX.get().expect(INIT_MSG).clone();
    msg_tx.send(Message::Update(key, extra)).unwrap();
}

pub fn for_each_error(f: fn(&MyError)) {
    let msg_tx = MSG_TX.get().expect(INIT_MSG).clone();
    msg_tx.send(Message::ForEach(f)).unwrap();
}

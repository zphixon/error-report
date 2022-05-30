#![doc(test(attr(warn(warnings))))]

//! Report errors concurrently.
//!
//! When you [report!] an error, that error is sent to an error collection thread (represented by
//! [ErrorThread]), and you receive a [key](DefaultKey) which corresponds to that error. Then you
//! can come back to that error later if you've realized something new, and add some additional
//! context.
//!
//! ```
//! let mut et = error_report::ErrorThread::default();
//! error_report::init(&mut et);
//!
//! let key = error_report::report!("dang");
//! // do some other stuff, maybe gather more information about that error
//!
//! let why = "something heinous";
//! error_report::update_error(key, format!("this is why: {why}"));
//!
//! error_report::for_each_error(|error| {
//!     println!("oh no: {error:?}");
//! });
//! ```

use {
    anyhow::Error,
    flume::{Receiver, RecvError, Sender},
    once_cell::sync::OnceCell,
    slotmap::SlotMap,
    std::thread::JoinHandle,
};

pub use slotmap::DefaultKey;

/// Report an error.
///
/// This macro is a thin shim around [anyhow::anyhow!]. Requires [init] to have been called.
///
/// # Panics
///
/// This macro will panic at runtime if [init] has not been called or [ErrorThread::done] has been
/// called.
///
/// # Examples
///
/// ```
/// # let mut et = error_report::ErrorThread::default();
/// # error_report::init(&mut et);
/// let key = error_report::report!("dang");
/// // do some other stuff, maybe gather more information about that error
/// let why = "something heinous";
/// error_report::update_error(key, format!("this is why: {why}"));
/// ```
#[macro_export]
macro_rules! report {
    ($e:expr) => {
        $crate::report_error(anyhow::anyhow!($e));
    };
}

/// The message which appears when the library is misused.
pub const INIT_MSG: &'static str = "init() should be called once, and its result not discarded.\nlet errors = error_report::init(); // do not assign to _, you must include a name";

/// Message types that the library may send to the error collector thread.
enum Message {
    /// An error that is reported.
    ///
    /// Requires a sender to be send along with it so that the error reporting thread may reply
    /// with the slotmap's key.
    Error(Error, Sender<DefaultKey>),

    /// Update an error.
    Update(DefaultKey, String),

    /// Execute a function for each error.
    ForEach(fn(&MyError)),

    /// Execute a function for each error, mutably.
    ForEachMut(fn(&mut MyError)),

    /// Exit the error collector thread.
    ///
    /// This is necessary because we hold onto a static [Sender], so the channel will never be
    /// closed under normal circumstances.
    Quit,
}

impl std::fmt::Debug for Message {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Message::Error(err, _) => write!(f, "Error({err:?})"),
            Message::Update(_, s) => write!(f, "Update({s:?})"),
            Message::ForEach(_) => write!(f, "ForEach(...)"),
            Message::ForEachMut(_) => write!(f, "ForEachMut(...)"),
            Message::Quit => write!(f, "Quit"),
        }
    }
}

unsafe impl Sync for Message {}
unsafe impl Send for Message {}

/// Wrapper type for [anyhow::Error].
#[derive(Debug)]
pub struct MyError {
    error: Error,
    extra: Option<String>,
}

impl MyError {
    /// Get the underlying [anyhow::Error].
    pub fn error(&self) -> &Error {
        &self.error
    }

    /// Get the extra information, if any.
    pub fn extra(&self) -> Option<&String> {
        self.extra.as_ref()
    }

    pub fn error_mut(&mut self) -> &mut Error {
        &mut self.error
    }

    pub fn extra_mut(&mut self) -> Option<&mut String> {
        self.extra.as_mut()
    }
}

/// The error collector thread.
///
/// A newtype wrapping [std::thread::JoinHandle]. Its [Drop] implementation stops the error
/// collector thread, meaning any library calls afterward will panic.
#[derive(Default)]
pub struct ErrorThread {
    handle: Option<JoinHandle<SlotMap<DefaultKey, MyError>>>,
}

impl ErrorThread {
    /// Get the final list of errors.
    ///
    /// There should be no more calls to library functions after this call.
    ///
    /// # Panics
    ///
    /// Panics if [init] has not been called.
    pub fn done(mut self) -> SlotMap<DefaultKey, MyError> {
        let tx = MSG_TX.get().expect(INIT_MSG).clone();
        tx.send(Message::Quit).expect(INIT_MSG);
        self.handle.take().unwrap().join().unwrap()
    }
}

impl Drop for ErrorThread {
    fn drop(&mut self) {
        let tx = MSG_TX.get().expect(INIT_MSG).clone();
        let _x = tx.send(Message::Quit);
    }
}

fn handle_messages(message_rx: Receiver<Message>) -> SlotMap<DefaultKey, MyError> {
    let mut errors = SlotMap::new();

    loop {
        let message = message_rx.recv();
        match message {
            Ok(Message::Error(error, sender)) => {
                let key = errors.insert(MyError { error, extra: None });
                sender.send(key).expect(INIT_MSG);
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

            Ok(Message::ForEachMut(f)) => {
                for (_, error) in errors.iter_mut() {
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

    errors
}

/// The [Sender] responsible for sending [Message]s to the error collector thread.
static MSG_TX: OnceCell<Sender<Message>> = OnceCell::new();

/// Initialize the error collector thread.
///
/// This is done as a non-associated function to require the user to not discard the [ErrorThread]
/// prematurely. This is important as its [Drop] implementation quits the error collector thread,
/// dropping the [Receiver] and thus causing any subsequent error reports to panic.
///
/// # Panics
///
/// The function panics if it has already been called.
///
/// # Examples
///
/// ```
/// let mut et = error_report::ErrorThread::default();
/// error_report::init(&mut et);
/// ```
pub fn init(error_thread: &mut ErrorThread) {
    let (message_tx, message_rx) = flume::unbounded();
    MSG_TX.set(message_tx).expect(INIT_MSG);

    let handle = std::thread::spawn(|| handle_messages(message_rx));

    error_thread.handle = Some(handle);
}

/// Report an error.
///
/// See also [report!].
///
/// # Panics
///
/// Panics if [init] has not been called or [ErrorThread::done] has been called.
pub fn report_error(error: Error) -> DefaultKey {
    // TODO are these clones necessary?
    let msg_tx = MSG_TX.get().expect(INIT_MSG).clone();
    let (key_tx, key_rx) = flume::bounded(1);
    msg_tx.send(Message::Error(error, key_tx)).expect(INIT_MSG);
    key_rx.recv().expect(INIT_MSG)
}

/// Update an error with additional information.
///
/// # Panics
///
/// Panics if [init] has not been called or [ErrorThread::done] has been called.
pub fn update_error(key: DefaultKey, extra: String) {
    let msg_tx = MSG_TX.get().expect(INIT_MSG).clone();
    msg_tx.send(Message::Update(key, extra)).expect(INIT_MSG);
}

/// Execute a function for each error.
///
/// # Panics
///
/// Panics if [init] has not been called or [ErrorThread::done] has been called.
pub fn for_each_error(f: fn(&MyError)) {
    let msg_tx = MSG_TX.get().expect(INIT_MSG).clone();
    msg_tx.send(Message::ForEach(f)).expect(INIT_MSG);
}

/// Execute a function for each error, mutably.
///
/// # Panics
///
/// Panics if [init] has not been called or [ErrorThread::done] has been called.
pub fn for_each_mut_error(f: fn(&mut MyError)) {
    let msg_tx = MSG_TX.get().expect(INIT_MSG).clone();
    msg_tx.send(Message::ForEachMut(f)).expect(INIT_MSG);
}

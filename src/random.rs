use error_report::{report, ErrorThread};

const NUM_ERRORS: usize = 5000;
const NUM_THREADS: usize = 1000;

fn main() {
    let mut et = ErrorThread::default();
    error_report::init(&mut et);

    let thread = || {
        for i in 0..NUM_ERRORS {
            report!(format!("error {i}"));
        }
    };

    let mut threads = Vec::new();
    for _ in 0..NUM_THREADS {
        threads.push(std::thread::spawn(thread));
    }

    for thread in threads {
        thread.join().unwrap();
    }

    for (_, err) in et.done() {
        println!("{err:?}");
    }
}

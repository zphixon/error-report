use std::io::Write;

const NUM_ERRORS_PER_THREAD: usize = 5000;
const NUM_THREADS: usize = 100;
const TOTAL_ERRORS: usize = NUM_THREADS * NUM_ERRORS_PER_THREAD;
const PROGRESS_WIDTH: usize = 60;

error_report::make_reporter!(Idk);

fn main() {
    let mut et = ErrorThread::default();
    Idk::init(&mut et);

    let (tx, rx) = flume::unbounded();
    let make_thread = |tx: Sender<()>| {
        return move || {
            for i in 0..NUM_ERRORS_PER_THREAD {
                report!(format!("{i}"));
                tx.send(()).unwrap();
            }
        };
    };

    let mut threads = Vec::new();
    for _ in 0..NUM_THREADS {
        threads.push(std::thread::spawn(make_thread(tx.clone())));
    }
    drop(tx); // so that the loop can exit

    let mut num_reports = 0;
    let mut prev_num_chars = 0;
    loop {
        if matches!(rx.recv(), Err(flume::RecvError::Disconnected)) {
            break;
        }
        num_reports += 1;

        let ratio = (num_reports as f64) / (TOTAL_ERRORS as f64);
        let num_chars = (PROGRESS_WIDTH as f64) * ratio;
        let num_chars_int = num_chars as usize;
        let bar_fill = "=".repeat(num_chars_int);
        let bar = format!("[{bar_fill:width$}]", width = PROGRESS_WIDTH);

        if prev_num_chars != num_chars_int || num_reports == 1 {
            let percent = (ratio * 100.0) as usize;
            print!("\r{num_reports}/{TOTAL_ERRORS} {percent:3}% {bar}");
            std::io::stdout().flush().unwrap();
        }
        prev_num_chars = num_chars_int;
    }
    println!();

    for thread in threads {
        thread.join().unwrap();
    }

    let errs = et.done();
    for (_, err) in errs.iter() {
        assert!(err.extra().is_none());
    }
    assert_eq!(errs.len(), TOTAL_ERRORS);
}

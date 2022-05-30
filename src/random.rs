const NUM_ERRORS: usize = 5000;
const NUM_THREADS: usize = 100;
const PROGRESS_WIDTH: usize = 40;

error_report::make_reporter!(Idk);

fn main() {
    let mut et = ErrorThread::default();
    Idk::init(&mut et);

    let (tx, rx) = flume::unbounded();
    let make_thread = |tx: Sender<()>| {
        return move || {
            for i in 0..NUM_ERRORS {
                report!(format!("{i}"));
                tx.send(()).unwrap();
            }
        };
    };

    let mut threads = Vec::new();
    for _ in 0..NUM_THREADS {
        threads.push(std::thread::spawn(make_thread(tx.clone())));
    }

    let total_reports = NUM_ERRORS * NUM_THREADS;
    let mut num_reports = 0;
    loop {
        rx.recv().unwrap();
        num_reports += 1;

        let num_chars = PROGRESS_WIDTH as f64 * (num_reports as f64 / total_reports as f64);
        let bar = "=".repeat(num_chars as usize);
        print!(
            "\r{num_reports}/{total_reports} [{bar:width$}]",
            width = PROGRESS_WIDTH
        );

        if num_reports == total_reports {
            break;
        }
    }
    println!();

    for thread in threads {
        thread.join().unwrap();
    }

    for (_, err) in et.done() {
        assert!(err.extra().is_none());
    }
}

const NUM_ERRORS: usize = 5000;
const NUM_THREADS: usize = 100;

error_report::make_reporter!(Idk<usize>);

fn main() {
    let mut et = ErrorThread::default();
    Idk::init(&mut et);

    let thread = || {
        for i in 0..NUM_ERRORS {
            let key = report!("");
            Idk::update(key, i);
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

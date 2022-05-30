error_report::make_reporter!(Idk<String>);

fn main() {
    let mut et = ErrorThread::default();
    init_reporter(&mut et);

    let t1 = std::thread::spawn(|| {
        report!("dang1");
        for_each_error(|error| {
            #[cfg(windows)]
            unsafe {
                windows::Win32::UI::WindowsAndMessaging::MessageBoxW(
                    None,
                    format!("oh no! {error:?}"),
                    "an error happened",
                    windows::Win32::UI::WindowsAndMessaging::MB_ICONERROR,
                );
            }
            println!("inner! {error:?}");
        });
    });

    let t2 = std::thread::spawn(|| {
        let key = report!("dang2");
        update_error(key, "extra stuff".to_string());
    });

    t1.join().unwrap();
    t2.join().unwrap();

    let errors = et.done();
    for (_key, error) in &errors {
        println!("{:?}", error);
    }
}

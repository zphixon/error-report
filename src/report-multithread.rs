fn main() {
    let t = error_report::init();

    let t1 = std::thread::spawn(|| {
        error_report::report!("dang1");
        error_report::for_each_error(|error| {
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
        let key = error_report::report!("dang2");
        error_report::update_error(key, "extra stuff".to_string());
    });

    t1.join().unwrap();
    t2.join().unwrap();

    let errors = t.done();
    for (_key, error) in &errors {
        println!("{:?}", error);
    }
}

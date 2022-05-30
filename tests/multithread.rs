error_report::make_reporter!(MyError<String>);

#[test]
fn test() {
    let mut et = ErrorThread::default();
    MyError::init(&mut et);

    let t1 = std::thread::spawn(|| {
        report!("dang1");
        MyError::for_each(|error| {
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
        MyError::update(key, "extra stuff".to_string());
    });

    t1.join().unwrap();
    t2.join().unwrap();

    let errors = et.done();
    for (_, error) in errors.iter() {
        assert!(format!("{:?}", error.error()).starts_with("dang"));
        if error.extra().is_some() {
            assert_eq!(error.extra().unwrap(), &String::from("extra stuff"));
        }
    }
    assert_eq!(errors.len(), 2);
}

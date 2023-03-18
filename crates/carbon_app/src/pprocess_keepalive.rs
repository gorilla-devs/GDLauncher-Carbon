fn scan_args_for_ppid() -> Option<u32> {
    let args = std::env::args().collect::<Vec<String>>();
    let mut ppid = None;
    for (i, arg) in args.iter().enumerate() {
        if arg == "--ppid" {
            ppid = Some(args[i + 1].parse::<u32>().unwrap());
        }
    }
    ppid
}

#[cfg(target_os = "windows")]
pub fn init() {
    use std::ptr;
    use winapi::shared::minwindef::DWORD;
    use winapi::um::minwinbase::STILL_ACTIVE;
    use winapi::um::processthreadsapi::{GetExitCodeProcess, OpenProcess};
    use winapi::um::winnt::{HANDLE, PROCESS_QUERY_LIMITED_INFORMATION};

    pub struct SendablePtr<T>(*mut T);
    unsafe impl<T> Send for SendablePtr<T> {}

    tokio::spawn(async move {
        let ppid: DWORD = scan_args_for_ppid().expect("Parent Pid not found");
        let parent =
            SendablePtr(unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, ppid) });

        if parent.0.is_null() {
            parent_exited();
        }

        loop {
            let mut exitcode: DWORD = 0;
            if unsafe { GetExitCodeProcess(parent.0, &mut exitcode) } != 0
                && exitcode != STILL_ACTIVE
            {
                std::process::exit(0);
            }
            tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;
        }
    });
}

#[cfg(target_os = "linux")]
pub fn init() {
    use libc::{prctl, sighandler_t, signal, PR_SET_PDEATHSIG, SIGUSR1};

    unsafe {
        signal(SIGUSR1, parent_exited as sighandler_t);
        prctl(PR_SET_PDEATHSIG, SIGUSR1);
    }
}

extern "C" fn parent_exited() {
    println!("Parent process exited");
    std::process::exit(0);
}

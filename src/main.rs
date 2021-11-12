use std::{
    collections::{HashMap, VecDeque},
    sync::mpsc::{channel, Receiver, Sender},
    time::Duration,
};

// the brains of the process
// this function takes an unordered heap of messages from all running threads and presents
// them in chronological order
fn printer_loop(items: Vec<(&str, usize)>, rx_print: Receiver<(String, Option<usize>)>) {
    // used to cache messages that are not ready to be displayed
    let mut cache: HashMap<String, Vec<Option<usize>>> = items
        .iter()
        .map(|x| (x.0.to_string(), Vec::<Option<usize>>::new()))
        .collect();

    // used to keep track of what crate we are currently on
    let mut queue: VecDeque<&str> = items.iter().map(|x| x.0).collect();

    let mut current_exe = queue.pop_front().unwrap();
    loop {
        // receive print messages in whatever order they arrive
        match rx_print.recv() {
            // somewhere in the middle of a stream of messages for an exe test crate
            Ok((exe, Some(num))) => {
                if exe == current_exe {
                    println!("{} - {}", exe, num);
                } else {
                    cache.get_mut(&exe).unwrap().push(Some(num));
                }
            }
            // no more messages will arrive from this exe test crate
            Ok((exe, None)) => {
                if exe == current_exe {
                    // loop through the cache and print as many messages as we can
                    // it could be that several crates have already finished
                    'mrloopy: loop {
                        match queue.pop_front() {
                            Some(new_exe) => current_exe = new_exe,
                            None => return,
                        }

                        for val in cache.get(current_exe).unwrap() {
                            match val {
                                Some(num) => println!("{} - {}", current_exe, num),
                                None => {
                                    continue 'mrloopy;
                                }
                            }
                        }

                        break;
                    }
                } else {
                    cache.get_mut(&exe).unwrap().push(None);
                }
            }

            Err(_) => {
                // we should never normally get here
                println!("done");
                break;
            }
        }
    }
}

fn run_test_crate(exe: String, sleep_ms: usize, tx_print: Sender<(String, Option<usize>)>) {
    let num_tests_in_crate = 5;
    for x in 0..num_tests_in_crate {
        // simulate a test doing work and taking time
        std::thread::sleep(Duration::from_millis(sleep_ms as u64));
        tx_print.send((exe.to_owned(), Some(x))).unwrap();
    }

    tx_print.send((exe, None)).unwrap();
}

fn main() {
    let items = vec![
        ("one", 250), // (exe_name, milliseconds)
        ("two", 200),
        ("three", 250),
        ("four", 600),
        ("five", 1000),
        ("six", 900),
        ("seven", 1600),
        ("eight", 200),
        ("nine", 500),
        ("ten", 1000),
    ];

    let (tx_done, rx_done) = channel::<()>();
    let (tx_print, rx_print) = channel::<(String, Option<usize>)>();

    let items_to_move = items.clone();
    let printer_handle = std::thread::spawn(move || printer_loop(items_to_move, rx_print));

    // every time a job is marked as done it frees up a spot for a new job to start
    // so marking num_jobs as done here effectively allows us to run test crates in parallel
    let num_jobs = 3;
    for _ in 0..num_jobs {
        tx_done.send(()).unwrap();
    }

    let mut run_test_handles = vec![];
    for item in items {
        let tx_print_clone = tx_print.clone();
        let tx_done_clone = tx_done.clone();

        let handle = std::thread::spawn(move || {
            // run the actual test crate
            run_test_crate(item.0.to_owned(), item.1, tx_print_clone);

            // mark this job as done so we can pick up another job
            tx_done_clone.send(()).unwrap()
        });

        run_test_handles.push(handle);

        // this blocks until jobs are marked as done
        rx_done.recv().unwrap();
    }

    // capture any crashed threads
    for handle in run_test_handles {
        handle.join().unwrap();
    }

    printer_handle.join().unwrap();
}

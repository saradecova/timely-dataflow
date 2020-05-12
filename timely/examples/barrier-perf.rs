extern crate timely;
extern crate hdrhist;

use std::rc::Rc;
use std::cell::RefCell;

use std::time::Instant;
use timely::dataflow::channels::pact::Pipeline;
use timely::dataflow::operators::{Feedback, ConnectLoop};
use timely::dataflow::operators::generic::operator::Operator;
use timely::logging::TimelyEvent;

fn main() {

    let iterations = std::env::args().nth(1).unwrap().parse::<usize>().unwrap_or(1_000_000);

    let tracker_log_filename = if let Ok(filename) = std::env::var("TRACKER_LOG_FILE") {
        Some(filename)
    } else {
        None
    };

    timely::execute_from_args(std::env::args().skip(2), move |worker| {
        if let Some(filename) = &tracker_log_filename {
            use std::io::Write;
            let tracker_log_file_1 = Rc::new(RefCell::new(
                std::fs::File::create(filename).expect("cannot open TRACKER_LOG_FILE")));

            worker.log_register().insert::<TimelyEvent,_>("timely", move |_, data| {
                data.drain(..).for_each(|x| {
                    writeln!(tracker_log_file_1.borrow_mut(), "{}", serde_json::to_string(&x.2).unwrap())
                    .expect("cannot write to TRACKER_LOG_FILE");
                })
            });
        }
        
        let index = worker.index();
    
        worker.dataflow(move |scope| {
            let (handle, stream) = scope.feedback::<usize>(1);
            let mut hist = hdrhist::HDRHist::new();
            let mut t0 = Instant::now();
            stream.unary_notify(
                Pipeline,
                "Barrier",
                vec![0],
                move |_, _, notificator| {
                    while let Some((cap, _count)) = notificator.next() {
                        let t1 = Instant::now();
                        let duration = t1.duration_since(t0);
                        hist.add_value(duration.as_secs() * 1_000_000_000u64 + duration.subsec_nanos() as u64);
                        t0 = t1;
                        let time = *cap.time() + 1;
                        if time < iterations {
                            notificator.notify_at(cap.delayed(&time));
                        } else {
                            if index == 0 {
                                println!("-------------\nSummary:\n{}", hist.summary_string());
                                println!("-------------\nCDF:");
                                for entry in hist.ccdf_upper_bound() {
                                    println!("{:?}", entry);
                                }
                            }
                        }
                    }
                }
            )
        .connect_loop(handle);
        });
    }).unwrap();
}

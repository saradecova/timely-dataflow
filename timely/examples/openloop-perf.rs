extern crate timely;
extern crate hdrhist;

use std::rc::Rc;
use std::cell::RefCell;

use timely::dataflow::{InputHandle, ProbeHandle};
use timely::dataflow::operators::{Input, Probe, Exchange};
use timely::logging::TimelyEvent;

fn main() {
    let tracker_log_filename = if let Ok(filename) = std::env::var("TRACKER_LOG_FILE") {
        Some(filename)
    } else {
        None
    };

    
    let mut args = std::env::args();
    args.next();
    let rate: usize = args.next().expect("Must specify rate").parse().expect("Rate must be an usize");
    let duration_s: usize = args.next().expect("Must specify duration_s").parse().expect("duration_s must be an usize");

    timely::execute_from_args(args, move |worker| {
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
        let peers = worker.peers();

        // re-synchronize all workers (account for start-up).
        timely::synchronization::Barrier::new(worker).wait();

        let timer = std::time::Instant::now();

        let mut input = InputHandle::new();
        let mut probe = ProbeHandle::new();

        // Create a dataflow that discards input data (just syncronizes).
        worker.dataflow(|scope| {
            scope
                .input_from(&mut input)     // read input.
                .exchange(move |x| *x as u64)
                .probe_with(&mut probe);    // observe output.
        });

        let ns_per_request = 1_000_000_000 / rate;
        let mut insert_counter = index;           // counts up as we insert records.
        let mut retire_counter = index;           // counts up as we retire records.

        let mut inserted_ns = 0;

        // We repeatedly consult the elapsed time, and introduce any data now considered available.
        // At the same time, we observe the output and record which inputs are considered retired.


        let mut hist = hdrhist::HDRHist::new();

        let mut counts = vec![[0u64; 16]; 64];

        let counter_limit = rate * duration_s;
        while retire_counter < counter_limit {

            // Open-loop latency-throughput test, parameterized by offered rate `ns_per_request`.
            let elapsed = timer.elapsed();
            let elapsed_ns = elapsed.as_secs() * 1_000_000_000 + (elapsed.subsec_nanos() as u64);

            // Determine completed ns.
            let acknowledged_ns: u64 = probe.with_frontier(|frontier| frontier[0]);

            // Notice any newly-retired records.
            while ((retire_counter * ns_per_request) as u64) < acknowledged_ns && retire_counter < counter_limit {
                let requested_at = (retire_counter * ns_per_request) as u64;
                let latency_ns = elapsed_ns - requested_at;
                let count_index = latency_ns.next_power_of_two().trailing_zeros() as usize;
                let low_bits = ((elapsed_ns - requested_at) >> (count_index - 5)) & 0xF;
                counts[count_index][low_bits as usize] += 1;

                hist.add_value(latency_ns);
                retire_counter += peers;
            }

                // Now, should we introduce more records before stepping the worker?
                // Three choices here:
                //
                //   1. Wait until previous batch acknowledged.
                //   2. Tick at most once every millisecond-ish.
                //   3. Geometrically increase outstanding batches.

                // Technique 1:
                // let target_ns = if acknowledged_ns >= inserted_ns { elapsed_ns } else { inserted_ns };

                // Technique 2:
                // let target_ns = elapsed_ns & !((1 << 20) - 1);

                // Technique 3:
                let target_ns = {
                    let delta: u64 = inserted_ns - acknowledged_ns;
                    let bits = ::std::mem::size_of::<u64>() * 8 - delta.leading_zeros() as usize;
                    let scale = ::std::cmp::max((1 << bits) / 4, 1024);                        
                    elapsed_ns & !(scale - 1)
                };

                // Common for each technique.
                if inserted_ns < target_ns {

                    while ((insert_counter * ns_per_request) as u64) < target_ns {
                        input.send(insert_counter);
                        insert_counter += 1;
                    }
                    input.advance_to(target_ns);
                    inserted_ns = target_ns;
                }

                worker.step();
            }


        // Report observed latency measurements.
        if index == 0 {
            println!("-------------\nSummary:\n{}", hist.summary_string());
            println!("-------------\nCDF:");
            for entry in hist.ccdf_upper_bound() {
                println!("{:?}", entry);
            }
        }
    }).unwrap();
}
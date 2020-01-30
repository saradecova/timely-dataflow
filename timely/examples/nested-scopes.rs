use timely::dataflow::Scope;
use timely::dataflow::operators::{Input, Inspect, Enter, Leave};
use timely::order::Product;
use timely::logging::{TimelyEvent};
use timely::progress::reachability::TrackerEvent;

fn main() {
    timely::execute_from_args(std::env::args(), move |worker| {
        worker.log_register().insert::<TimelyEvent,_>("timely", |_, data|
            data.iter().for_each(|x| println!("{}", serde_json::to_string(&x.2).unwrap()))
        );

        worker.log_register().insert::<TrackerEvent,_>("timely/tracker", |_, data|
            data.iter().for_each(|x| println!("{}", serde_json::to_string(&x.2).unwrap()))
        );

        // must specify types as nothing else drives inference.
        let _input = worker.dataflow::<u64,_,_>(|child1| {
            let (input, stream) = child1.new_input::<String>();
            let _output = child1.scoped::<Product<u64,u32>,_,_>("ScopeName", |child2| {
                stream
                    .enter(child2)
                    .inspect(move |x| println!("hello {}", x))
                    .inspect(move |x| println!("hello {}", x))
                    .leave()
            });
            input
        });
    }).unwrap();
}
use timely::dataflow::Scope;
use timely::dataflow::operators::{
    Input, Inspect, Enter, Leave, Feedback,
    feedback::ConnectLoop,
    generic::operator::Operator,
    concat::Concat};
use timely::dataflow::channels::pact::Pipeline;
use timely::order::Product;
use timely::logging::{TimelyEvent};
use timely::progress::reachability::TrackerEvent;
use timely::progress::frontier::Antichain;

fn main() {
    timely::execute_from_args(std::env::args(), move |worker| {
        worker.log_register().insert::<TimelyEvent,_>("timely", |_, data|
            data.iter().for_each(|x| println!("{:?}", x.2))
        );

        worker.log_register().insert::<TrackerEvent,_>("timely/tracker", |_, data|
            data.iter().for_each(|x| println!("{:?}", x.2))
        );

        // must specify types as nothing else drives inference.
        let _input = worker.dataflow::<u64,_,_>(|scope| {
            let (input, stream) = scope.new_input::<()>();
            let _output = scope.scoped::<Product<u64,u64>,_,_>("ScopeName", |child| {
                let (inside_loop_handle, loop_stream) =
                    child.feedback::<()>(Product {outer: 0, inner: 1});

                let out = stream
                    .enter(child)
                    .concat(&loop_stream)
                    .unary_frontier(Pipeline, "incomparable", |default_cap, _info| {
                        let mut cap1 = Some(default_cap); // None
                        let mut caps = Antichain::new();
                        move |input, output| {
                            while let Some((time, data)) = input.next() { }
                            if let Some(cap) = cap1.take() {
                                caps.insert(cap.delayed(&Product { outer: 3, inner: 1})); // CM
                                caps.insert(cap.delayed(&Product { outer: 1, inner: 3})); // CM
                                // cap goes out of scope here
                            }
                            let frontier = input.frontier();
                            eprintln!("{:?}", frontier);
                            if (!frontier.less_than(&Product { outer: 3, inner: 2 }) &&
                                !frontier.less_than(&Product { outer: 1, inner: 4 })) {
                                caps.clear();
                            }
                        }
                    });

                out.connect_loop(inside_loop_handle);
                out.leave()
            });
            input
        });
    }).unwrap();
}

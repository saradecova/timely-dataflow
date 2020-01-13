/* Two nested scopes in sequence.
   The first inner scope contains a loop.
*/

use timely::dataflow::Scope;
use timely::dataflow::operators::{Concat, ConnectLoop, Input, Inspect, Enter, Leave, Map, Partition};
use timely::order::Product;
use timely::logging::{TimelyEvent};
use timely::progress::reachability::TrackerEvent;
use timely::dataflow::operators::feedback::Feedback;

fn main() {
    let mut args = std::env::args();
    args.next();
    timely::execute_from_args(std::env::args(), move |worker| {
        worker.log_register().insert::<TimelyEvent,_>("timely", |_, data|
            data.iter().for_each(|x| println!("{:?}", x.2))
        );

        worker.log_register().insert::<TrackerEvent,_>("timely/tracker", |_, data|
            data.iter().for_each(|x| println!("{:?}", x.2))
        );

        // must specify types as nothing else drives inference.
        let mut input = worker.dataflow::<u64,_,_>(|child1| {
            let (input, stream) = child1.new_input::<u64>();
            let output = child1.scoped::<Product<u64,u32>,_,_>("First Inner Scope", |child2| {
                let (inside_loop_handle, loop_stream) =
                    child2.feedback(Product {outer: 0, inner: 1});

                let branches =
                stream
                    .enter(child2)
                    .concat(&loop_stream)
                    .map(|x| if x % 2 == 0 { x / 2 } else { 3 * x + 1 })
                    .partition(2, |x| if x > 2 { (0, x) } else { (1, x) });
                
                branches[0].connect_loop(inside_loop_handle);
                branches[1]
                    .leave()
                    .inspect(move |x| println!("Leaving first subscope with {}", x))
            });

            child1.scoped::<Product<u64,u32>,_,_>("Second Inner Scope", |child3| {
                output
                    .enter(child3)
                    .inspect(move |x| println!("Inside second subscope with {}", x))
                    .leave()
            });
            input
        });

        for round in 0..2 {
            input.send(round);
            input.advance_to(round);

        };

    }).unwrap();
}
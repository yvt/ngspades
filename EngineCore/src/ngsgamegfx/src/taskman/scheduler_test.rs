//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use crate::taskman;

#[derive(Debug, Clone, Copy)]
struct MyCell(usize);

#[derive(Debug)]
struct MyTask {
    producing: Vec<taskman::CellId>,
    consuming: Vec<taskman::CellId>,
}

impl taskman::Task for MyTask {
    fn execute(&self, context: &taskman::Context) {
        println!(
            "producing: [{:?}]",
            self.producing
                .iter()
                .map(|&i| *context.borrow_cell_mut(i).downcast_ref::<MyCell>().unwrap())
                .collect::<Vec<_>>()
        );
        println!(
            "consuming: [{:?}]",
            self.consuming
                .iter()
                .map(|&i| *context.borrow_cell(i).downcast_ref::<MyCell>().unwrap())
                .collect::<Vec<_>>()
        );
    }
}

#[test]
fn test() {
    let mut builder = taskman::GraphBuilder::new();

    let cell0 = builder.define_cell(MyCell(1));
    let cell1 = builder.define_cell(MyCell(2));
    let cell2 = builder.define_cell(MyCell(3));

    builder.define_task(taskman::TaskInfo {
        cell_uses: vec![cell0.use_as_producer()],
        task: Box::new(MyTask {
            producing: vec![cell0],
            consuming: vec![],
        }),
    });

    builder.define_task(taskman::TaskInfo {
        cell_uses: vec![cell0.use_as_consumer(), cell1.use_as_producer()],
        task: Box::new(MyTask {
            producing: vec![cell1],
            consuming: vec![cell0],
        }),
    });

    builder.define_task(taskman::TaskInfo {
        cell_uses: vec![
            cell0.use_as_consumer(),
            cell1.use_as_consumer(),
            cell2.use_as_producer(),
        ],
        task: Box::new(MyTask {
            producing: vec![cell2],
            consuming: vec![cell0, cell1],
        }),
    });

    let mut graph = builder.build();
    println!("{:#?}", graph);

    let executor = xdispatch::Queue::global(xdispatch::QueuePriority::Default);

    graph.run(&executor);
}

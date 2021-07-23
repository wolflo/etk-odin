use etk_asm::ingest::Ingest;
use evmodin::{*, host::DummyHost, tracing::NoopTracer};
use ethereum_types::{Address, U256};
use hex_literal::hex;

#[tokio::main]
async fn main() {
    // Bytecode to run.
    // Note that testing this snippet directly requires control of stack, memory, and returndata.
    // It expects a memory pointer to an arbitrary prefix on the stack, followed
    // by the prefix's length. It should return the prefix ++ forwarded returndata.
    let opcodes = r#"
        returndatasize
        push1 0x00
        dup4
        dup4
        add
        returndatacopy
        swap1
        returndatasize
        add
        swap1
        return
    "#;

    // Assemble code with etk-asm
    let mut code = Vec::new();
    let mut ingest = Ingest::new(&mut code);
    ingest.ingest("", &opcodes).unwrap();

    // Prepare a message to execute with evmodin
    let message = Message {
        kind: CallKind::Call,
        is_static: true,
        depth: 0,
        gas: 200,
        destination: Address::zero(),
        sender: Address::zero(),
        input_data: vec![].into(),
        value: U256::zero(),
    };

    let state = &mut ExecutionState::new(message.clone(), Revision::London);

    // Set initial stack
    // [ bottom, top ]
    let starting_stack = vec![ 0x04, 0x01 ];
    let stack = state.stack_mut();
    for word in starting_stack {
        stack.push(word.into());
    }

    // Set initial memory
    let starting_mem = vec![ 0x00, 0xba, 0xdf, 0x00, 0xd0 ];
    *state.memory_mut() = starting_mem.to_vec();

    // Set initial returndata (i.e. what's used for RETURNDATA* ops)
    let starting_return_data = vec![ 0xba, 0xdc, 0xaf, 0xe0 ];
    *state.return_data_mut() = starting_return_data.into();

    // Execute the message with our modified initial state
    let host = &mut DummyHost;
    let tracer =  &mut NoopTracer;
    let analyzed = AnalyzedCode::analyze(code);
    let result = analyzed.execute_with_state_ext(host, tracer, state).await;

    let expected =
        Output {
            status_code: StatusCode::Success,
            gas_left: 166,
            output_data: hex!("badf00d0badcafe0").to_vec().into(),
            create_address: None,
        };

    assert_eq!(result.unwrap(), expected);
}

// bytecode stack annotation
//                 //[ ptr prefix_len ]
// returndatasize  //[ returndatasize ptr prefix_len ]
// push1 0x00      //[ 0x00 returndatasize ptr prefix_len ]
// dup4            //[ prefix_len 0x00 returndatasize ptr prefix_len ]
// dup4            //[ ptr prefix_len 0x00 returndatasize ptr prefix_len ]
// add             //[ ptr+prefix_len 0x00 returndatasize ptr prefix_len ]
// returndatacopy  //[ ptr prefix_len ]
// swap1           //[ prefix_len ptr ]
// returndatasize  //[ returndatasize prefix_len ptr ]
// add             //[ returndatasize+prefix_len ptr]
// swap1           //[ ptr returndatasize+prefix_len]
// return


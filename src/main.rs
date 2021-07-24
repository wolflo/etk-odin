use etk_asm::ingest::Ingest;
use evmodin::{
    *,
    tracing::NoopTracer,
    util::mocked_host::MockedHost
};
use ethereum_types::{Address, U256, H256};
use hex_literal::hex;


#[tokio::main]
async fn main() {
    // The code in code.etk is arbitrary other than the fact that testing
    // it directly requires control of stack, memory, storage, and returndata.
    // It expects 3 values on the stack:
    // 1. A pointer to an arbitrary length "return prefix" in memory
    // 2. The length of the prefix
    // 3. A storage slot
    // It returns the prefix ++ sload(slot) ++ forwarded returndata

    // Assemble code with etk-asm
    let mut code = Vec::new();
    let mut ingest = Ingest::new(&mut code);
    ingest.ingest_file("./src/code.etk").unwrap();

    // Prepare a message to execute with evmodin
    let message = Message {
        kind: CallKind::Call,
        is_static: true,
        depth: 0,
        gas: 3000,
        destination: Address::zero(),
        sender: Address::zero(),
        input_data: vec![].into(),
        value: U256::zero(),
    };

    let state = &mut ExecutionState::new(message.clone(), Revision::London);

    // Set initial stack
    // [ bottom, top ]
    let initial_stack: [U256; 3] = [ 0x42.into(), 0x04.into(), 0x01.into() ];
    state.stack_mut().0.try_extend_from_slice(&initial_stack).unwrap();

    // Set initial memory
    let initial_mem = vec![ 0x00, 0xBA, 0xDF, 0x00, 0xD0 ];
    *state.memory_mut() = initial_mem;

    // Set initial returndata (i.e. what's used for RETURNDATA* ops)
    let initial_return_data = vec![ 0xBA, 0xDC, 0xAF, 0xE0 ];
    *state.return_data_mut() = initial_return_data.into();

    // Set initial storage (key, value)
    let initial_storage = [ (0x42, 0xDEADBEEF as u32) ];
    let host = &mut MockedHost::default();
    for store in initial_storage {
        host.set_storage(
            message.destination,
            H256(U256::from(store.0).into()),
            H256(U256::from(store.1).into())
        ).await.expect("failed to initialize storage");
    };

    // Execute the message with our modified initial state
    let tracer =  &mut NoopTracer;
    let analyzed = AnalyzedCode::analyze(code);
    let result = analyzed.execute_with_state_ext(host, tracer, state).await;

    // Should return the prefix preset in memory ++ the preset storage slot value
    // ++ the preset returndata
    let expected_output = hex!("
        BADF00D0
        00000000000000000000000000000000000000000000000000000000DEADBEEF
        BADCAFE0
    ");

    let expected =
        Output {
            status_code: StatusCode::Success,
            gas_left: 839,
            output_data: expected_output.to_vec().into(),
            create_address: None,
        };

    assert_eq!(result.unwrap(), expected);

    // Can also make assertions about the final execution state
    assert_eq!(state.stack_mut().len(), 2);
}

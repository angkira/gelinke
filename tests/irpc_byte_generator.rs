/// Test utility to generate exact iRPC message bytes for Renode testing
///
/// This generates postcard-serialized iRPC messages that can be used in
/// Python test scripts to ensure byte-level compatibility.

use irpc::protocol::{Header, Message, Payload, SetTargetPayload};

fn print_message_bytes(name: &str, msg: &Message) {
    let bytes = msg.serialize().expect("Serialization failed");
    
    println!("{:30} [{}] bytes", name, bytes.len());
    print!("  Hex:   ");
    for (i, b) in bytes.iter().enumerate() {
        if i > 0 && i % 8 == 0 {
            print!(" ");
        }
        print!("{:02X} ", b);
    }
    println!();
    
    print!("  Robot: ");
    for b in bytes.iter() {
        print!("0x{:02X} ", b);
    }
    println!("\n");
}

#[test]
fn generate_test_messages() {
    println!("\n=== iRPC Message Byte Sequences ===\n");
    
    // 1. Configure
    let configure = Message {
        header: Header {
            source_id: 0x0000,
            target_id: 0x0010,
            msg_id: 1,
        },
        payload: Payload::Configure,
    };
    print_message_bytes("Configure", &configure);
    
    // 2. Activate
    let activate = Message {
        header: Header {
            source_id: 0x0000,
            target_id: 0x0010,
            msg_id: 2,
        },
        payload: Payload::Activate,
    };
    print_message_bytes("Activate", &activate);
    
    // 3. Deactivate
    let deactivate = Message {
        header: Header {
            source_id: 0x0000,
            target_id: 0x0010,
            msg_id: 3,
        },
        payload: Payload::Deactivate,
    };
    print_message_bytes("Deactivate", &deactivate);
    
    // 4. Reset
    let reset = Message {
        header: Header {
            source_id: 0x0000,
            target_id: 0x0010,
            msg_id: 4,
        },
        payload: Payload::Reset,
    };
    print_message_bytes("Reset", &reset);
    
    // 5. SetTarget
    let set_target = Message {
        header: Header {
            source_id: 0x0000,
            target_id: 0x0010,
            msg_id: 5,
        },
        payload: Payload::SetTarget(SetTargetPayload {
            target_angle: 90.0,
            velocity_limit: 150.0,
        }),
    };
    print_message_bytes("SetTarget(90°, 150°/s)", &set_target);
    
    // 6. ArmReady (broadcast)
    let arm_ready = Message {
        header: Header {
            source_id: 0x0000,
            target_id: 0xFFFF,
            msg_id: 0,
        },
        payload: Payload::ArmReady,
    };
    print_message_bytes("ArmReady (broadcast)", &arm_ready);
    
    println!("=== Copy these byte sequences to Python helper ===");
}

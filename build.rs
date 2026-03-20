fn main() {
    // Generate tonic gRPC clients from the shared proto definitions.
    //
    // MVP target:
    // - AuthService
    // - KeyService
    // - MessagingService
    //
    // Additional imports (core/envelope.proto, etc.) are pulled transitively.
    let proto_root = "../construct-protos";

    let protos = [
        format!("{proto_root}/services/auth_service.proto"),
        format!("{proto_root}/services/key_service.proto"),
        format!("{proto_root}/services/messaging_service.proto"),
        format!("{proto_root}/core/envelope.proto"),
    ];

    tonic_build::configure()
        .build_client(true)
        .compile(&protos.iter().map(|p| p.as_str()).collect::<Vec<_>>(), &[proto_root])
        .expect("proto compilation failed");
}


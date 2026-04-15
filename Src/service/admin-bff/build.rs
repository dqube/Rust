fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_prost_build::configure()
        .build_server(false) // client only — we call downstream services, not serve them
        .build_client(true)
        .type_attribute(".", "#[derive(serde::Serialize, serde::Deserialize, utoipa::ToSchema)]")
        .field_attribute(".", "#[serde(default)]")
        .compile_protos(
            &["proto/product.proto", "proto/order.proto"],
            &["proto"],
        )?;
    Ok(())
}

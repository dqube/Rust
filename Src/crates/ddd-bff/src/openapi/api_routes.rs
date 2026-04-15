//! Declarative endpoint catalogue for BFF gateways.
//!
//! Each REST endpoint exposed by a BFF falls into one of three buckets —
//! pass-through to a downstream gRPC service, fan-out aggregation, or
//! server-sent-event streaming. Describe them all through one [`ApiRoute`]
//! table; [`inject_routes`] then renders the corresponding OpenAPI 3.x
//! `paths` entries into a Scalar-ready spec.
//!
//! This keeps the OpenAPI documentation for transcoded endpoints in sync
//! with the route table without scattering `#[utoipa::path]` attributes
//! (or hand-built JSON blobs) across the codebase. Adding a new endpoint
//! becomes a one-line addition to the table.
//!
//! Schemas referenced by `schema_ref` must already exist in
//! `components.schemas` — typically because the request/response types
//! derive `utoipa::ToSchema` and are listed on the umbrella
//! `#[derive(OpenApi)]` struct, or because `merged_openapi` has folded
//! them in from a downstream service spec.

use serde_json::{json, Value};

/// The transcoding pattern an endpoint uses.
#[derive(Debug, Clone, Copy)]
pub enum RouteKind {
    /// REST → single gRPC unary call → JSON.
    Passthrough {
        /// Logical upstream name (must match `GrpcClientPool` registration).
        upstream: &'static str,
        /// Fully-qualified `package.Service/Method`.
        grpc_method: &'static str,
    },
    /// REST → multi-call fan-out → JSON. Implementation is provided by
    /// a hand-written aggregator handler.
    Aggregation,
    /// REST → gRPC server-streaming → SSE.
    Stream {
        upstream: &'static str,
        grpc_method: &'static str,
    },
}

/// A single OpenAPI parameter (path / query / header).
#[derive(Debug, Clone, Copy)]
pub struct Param {
    pub name: &'static str,
    /// `"path"`, `"query"`, or `"header"`.
    pub location: &'static str,
    pub required: bool,
    /// JSON Schema primitive type — `"string"`, `"integer"`, etc.
    pub schema_type: &'static str,
    pub description: &'static str,
}

/// A reference to a schema defined under `components.schemas`.
#[derive(Debug, Clone, Copy)]
pub struct SchemaRef {
    pub name: &'static str,
    /// e.g. `"application/json"` or `"text/event-stream"`.
    pub content_type: &'static str,
}

/// A single response entry.
#[derive(Debug, Clone, Copy)]
pub struct ResponseSpec {
    pub status: u16,
    pub description: &'static str,
    pub schema: Option<SchemaRef>,
}

/// A declarative description of one REST endpoint.
#[derive(Debug, Clone, Copy)]
pub struct ApiRoute {
    pub kind: RouteKind,
    /// HTTP method — `"GET"`, `"POST"`, …
    pub method: &'static str,
    /// OpenAPI-style path with `{name}` placeholders.
    pub path: &'static str,
    pub operation_id: &'static str,
    pub summary: &'static str,
    pub tag: &'static str,
    pub params: &'static [Param],
    pub request_body: Option<SchemaRef>,
    pub responses: &'static [ResponseSpec],
}

// ─── OpenAPI rendering ──────────────────────────────────────────────────────

/// Inject every route in `routes` into `spec`'s `paths` object.
///
/// Existing entries at the same `(path, method)` are overwritten.
pub fn inject_routes(spec: &mut Value, routes: &[ApiRoute]) {
    let paths = paths_object(spec);

    for route in routes {
        let entry = paths
            .entry(route.path.to_string())
            .or_insert_with(|| json!({}));

        if let Some(obj) = entry.as_object_mut() {
            obj.insert(route.method.to_lowercase(), build_operation(route));
        }
    }
}

fn paths_object(spec: &mut Value) -> &mut serde_json::Map<String, Value> {
    let root = spec
        .as_object_mut()
        .expect("OpenAPI spec must be a JSON object");
    root.entry("paths".to_string())
        .or_insert_with(|| json!({}))
        .as_object_mut()
        .expect("paths must be an object")
}

fn build_operation(route: &ApiRoute) -> Value {
    let mut op = serde_json::Map::new();

    op.insert("tags".into(), json!([route.tag]));
    op.insert("operationId".into(), json!(route.operation_id));
    op.insert("summary".into(), json!(route.summary));
    op.insert("description".into(), json!(describe_kind(&route.kind)));

    // x- extensions for tooling that wants to introspect the kind.
    op.insert("x-bff-kind".into(), json!(kind_tag(&route.kind)));
    if let Some((upstream, method)) = upstream_of(&route.kind) {
        op.insert("x-bff-upstream".into(), json!(upstream));
        op.insert("x-bff-grpc-method".into(), json!(method));
    }

    if !route.params.is_empty() {
        op.insert(
            "parameters".into(),
            json!(route.params.iter().map(param_value).collect::<Vec<_>>()),
        );
    }

    if let Some(body) = route.request_body.as_ref() {
        op.insert(
            "requestBody".into(),
            json!({
                "required": true,
                "content": { body.content_type: { "schema": schema_ref(body.name) } }
            }),
        );
    }

    let mut responses = serde_json::Map::new();
    for resp in route.responses {
        let mut entry = serde_json::Map::from_iter([(
            "description".to_string(),
            json!(resp.description),
        )]);
        if let Some(schema) = resp.schema.as_ref() {
            entry.insert(
                "content".into(),
                json!({ schema.content_type: { "schema": schema_ref(schema.name) } }),
            );
        }
        responses.insert(resp.status.to_string(), Value::Object(entry));
    }
    op.insert("responses".into(), Value::Object(responses));

    Value::Object(op)
}

fn param_value(p: &Param) -> Value {
    json!({
        "name": p.name,
        "in": p.location,
        "required": p.required,
        "description": p.description,
        "schema": { "type": p.schema_type },
    })
}

fn schema_ref(name: &str) -> Value {
    json!({ "$ref": format!("#/components/schemas/{name}") })
}

fn kind_tag(k: &RouteKind) -> &'static str {
    match k {
        RouteKind::Passthrough { .. } => "passthrough_unary",
        RouteKind::Aggregation => "aggregation",
        RouteKind::Stream { .. } => "passthrough_stream",
    }
}

fn upstream_of(k: &RouteKind) -> Option<(&'static str, &'static str)> {
    match k {
        RouteKind::Passthrough {
            upstream,
            grpc_method,
        }
        | RouteKind::Stream {
            upstream,
            grpc_method,
        } => Some((upstream, grpc_method)),
        RouteKind::Aggregation => None,
    }
}

fn describe_kind(k: &RouteKind) -> String {
    match k {
        RouteKind::Passthrough {
            upstream,
            grpc_method,
        } => {
            format!(
                "Pass-through: forwarded to `{upstream}` gRPC method `{grpc_method}`."
            )
        }
        RouteKind::Aggregation => {
            "Aggregation: parallel fan-out to multiple downstream calls.".to_string()
        }
        RouteKind::Stream {
            upstream,
            grpc_method,
        } => {
            format!(
                "Server-sent events: streamed from `{upstream}` gRPC method `{grpc_method}`."
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ROUTES: &[ApiRoute] = &[
        ApiRoute {
            kind: RouteKind::Passthrough {
                upstream: "product",
                grpc_method: "product.v1.ProductService/GetProduct",
            },
            method: "GET",
            path: "/admin/products/{id}",
            operation_id: "get_product",
            summary: "Fetch a product by id",
            tag: "Products",
            params: &[Param {
                name: "id",
                location: "path",
                required: true,
                schema_type: "string",
                description: "Product id",
            }],
            request_body: None,
            responses: &[
                ResponseSpec {
                    status: 200,
                    description: "Found",
                    schema: Some(SchemaRef {
                        name: "GetProductResponse",
                        content_type: "application/json",
                    }),
                },
                ResponseSpec {
                    status: 404,
                    description: "Not found",
                    schema: Some(SchemaRef {
                        name: "ProblemDetail",
                        content_type: "application/problem+json",
                    }),
                },
            ],
        },
        ApiRoute {
            kind: RouteKind::Aggregation,
            method: "POST",
            path: "/admin/orders/batch",
            operation_id: "batch_get_orders",
            summary: "Fetch many orders in parallel",
            tag: "Aggregation",
            params: &[],
            request_body: Some(SchemaRef {
                name: "BatchRequest",
                content_type: "application/json",
            }),
            responses: &[ResponseSpec {
                status: 200,
                description: "Per-id results",
                schema: Some(SchemaRef {
                    name: "BatchResponse",
                    content_type: "application/json",
                }),
            }],
        },
    ];

    #[test]
    fn injects_paths_and_operations() {
        let mut spec = json!({ "openapi": "3.0.3", "paths": {} });
        inject_routes(&mut spec, ROUTES);

        let get = &spec["paths"]["/admin/products/{id}"]["get"];
        assert_eq!(get["operationId"], "get_product");
        assert_eq!(get["x-bff-kind"], "passthrough_unary");
        assert_eq!(get["x-bff-upstream"], "product");
        assert_eq!(get["parameters"][0]["name"], "id");
        assert_eq!(
            get["responses"]["200"]["content"]["application/json"]["schema"]["$ref"],
            "#/components/schemas/GetProductResponse"
        );

        let post = &spec["paths"]["/admin/orders/batch"]["post"];
        assert_eq!(post["x-bff-kind"], "aggregation");
        assert!(post.get("x-bff-upstream").is_none());
        assert_eq!(
            post["requestBody"]["content"]["application/json"]["schema"]["$ref"],
            "#/components/schemas/BatchRequest"
        );
    }

    #[test]
    fn paths_section_is_created_if_missing() {
        let mut spec = json!({ "openapi": "3.0.3" });
        inject_routes(&mut spec, ROUTES);
        assert!(spec["paths"].is_object());
    }
}

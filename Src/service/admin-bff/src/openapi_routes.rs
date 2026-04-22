use ddd_bff::openapi::{ApiRoute, Param, ResponseSpec, RouteKind, SchemaRef};

/// Declarative table of every BFF endpoint that should appear in the
/// merged OpenAPI document with `x-bff-kind` metadata. Adding a new
/// pass-through, aggregation, or streaming endpoint is a one-line
/// addition here — no per-handler `#[utoipa::path]` boilerplate and no
/// hand-rolled JSON injection required.
///
/// Schemas referenced below must already exist in `components.schemas`,
/// either because they derive `utoipa::ToSchema` (and are registered on
/// `AdminApiDoc` or via proto build-time derives) or because
/// `merged_openapi` folded them in from a downstream service spec.
pub const API_ROUTES: &[ApiRoute] = &[
    // ── Pass-through (REST → gRPC) ─────────────────────────────────────
    ApiRoute {
        kind: RouteKind::Passthrough {
            upstream: "product",
            grpc_method: "product.v1.ProductService/CreateProduct",
        },
        method: "POST",
        path: "/admin/products",
        operation_id: "create_product",
        summary: "Create a product",
        tag: "Products",
        params: &[],
        request_body: Some(SchemaRef {
            name: "CreateProductRequest",
            content_type: "application/json",
        }),
        responses: &[
            ResponseSpec {
                status: 201,
                description: "Created",
                schema: Some(SchemaRef {
                    name: "CreateProductResponse",
                    content_type: "application/json",
                }),
            },
            ResponseSpec {
                status: 400,
                description: "Validation error",
                schema: Some(SchemaRef {
                    name: "ProblemDetail",
                    content_type: "application/problem+json",
                }),
            },
        ],
    },
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
            description: "Product id (UUID)",
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
        kind: RouteKind::Passthrough {
            upstream: "product",
            grpc_method: "product.v1.ProductService/ListProducts",
        },
        method: "GET",
        path: "/admin/products",
        operation_id: "list_products",
        summary: "List products with pagination",
        tag: "Products",
        params: &[
            Param {
                name: "page",
                location: "query",
                required: false,
                schema_type: "integer",
                description: "Page number (default: 1)",
            },
            Param {
                name: "per_page",
                location: "query",
                required: false,
                schema_type: "integer",
                description: "Items per page (default: 20)",
            },
        ],
        request_body: None,
        responses: &[ResponseSpec {
            status: 200,
            description: "Page of products",
            schema: Some(SchemaRef {
                name: "ListProductsResponse",
                content_type: "application/json",
            }),
        }],
    },
    ApiRoute {
        kind: RouteKind::Passthrough {
            upstream: "product",
            grpc_method: "product.v1.ProductService/UpdateStock",
        },
        method: "PUT",
        path: "/admin/products/{id}/stock",
        operation_id: "update_stock",
        summary: "Update a product's stock level",
        tag: "Products",
        params: &[Param {
            name: "id",
            location: "path",
            required: true,
            schema_type: "string",
            description: "Product id (UUID)",
        }],
        request_body: Some(SchemaRef {
            name: "UpdateStockRequest",
            content_type: "application/json",
        }),
        responses: &[ResponseSpec {
            status: 200,
            description: "Updated",
            schema: None,
        }],
    },
    ApiRoute {
        kind: RouteKind::Passthrough {
            upstream: "product",
            grpc_method: "product.v1.ProductService/DeactivateProduct",
        },
        method: "PUT",
        path: "/admin/products/{id}/deactivate",
        operation_id: "deactivate_product",
        summary: "Deactivate a product",
        tag: "Products",
        params: &[Param {
            name: "id",
            location: "path",
            required: true,
            schema_type: "string",
            description: "Product id (UUID)",
        }],
        request_body: None,
        responses: &[ResponseSpec {
            status: 200,
            description: "Deactivated",
            schema: None,
        }],
    },
    // ── Aggregation (multi-call fan-out) ───────────────────────────────
    ApiRoute {
        kind: RouteKind::Aggregation,
        method: "GET",
        path: "/admin/catalog/summary",
        operation_id: "get_catalog_summary",
        summary: "Aggregate product counts and recent items",
        tag: "Aggregation",
        params: &[],
        request_body: None,
        responses: &[ResponseSpec {
            status: 200,
            description: "Aggregated catalog summary",
            schema: None,
        }],
    },
    ApiRoute {
        kind: RouteKind::Aggregation,
        method: "POST",
        path: "/admin/orders/batch",
        operation_id: "batch_get_orders",
        summary: "Fetch many orders in parallel (partial-failure tolerant)",
        tag: "Aggregation",
        params: &[],
        request_body: Some(SchemaRef {
            name: "BatchRequest",
            content_type: "application/json",
        }),
        responses: &[
            ResponseSpec {
                status: 200,
                description: "Per-id results",
                schema: Some(SchemaRef {
                    name: "BatchResponse",
                    content_type: "application/json",
                }),
            },
            ResponseSpec {
                status: 400,
                description: "Too many ids",
                schema: Some(SchemaRef {
                    name: "ProblemDetail",
                    content_type: "application/problem+json",
                }),
            },
        ],
    },
    // ── Orders (REST → gRPC pass-through) ─────────────────────────────
    ApiRoute {
        kind: RouteKind::Passthrough {
            upstream: "order",
            grpc_method: "order.v1.OrderService/CreateOrder",
        },
        method: "POST",
        path: "/admin/orders",
        operation_id: "create_order",
        summary: "Place a new order",
        tag: "Orders",
        params: &[],
        request_body: Some(SchemaRef {
            name: "CreateOrderRequest",
            content_type: "application/json",
        }),
        responses: &[
            ResponseSpec {
                status: 201,
                description: "Created",
                schema: Some(SchemaRef {
                    name: "CreateOrderResponse",
                    content_type: "application/json",
                }),
            },
            ResponseSpec {
                status: 400,
                description: "Validation error",
                schema: Some(SchemaRef {
                    name: "ProblemDetail",
                    content_type: "application/problem+json",
                }),
            },
        ],
    },
    ApiRoute {
        kind: RouteKind::Passthrough {
            upstream: "order",
            grpc_method: "order.v1.OrderService/GetOrder",
        },
        method: "GET",
        path: "/admin/orders/{id}",
        operation_id: "get_order",
        summary: "Fetch an order by id",
        tag: "Orders",
        params: &[Param {
            name: "id",
            location: "path",
            required: true,
            schema_type: "string",
            description: "Order id (UUID)",
        }],
        request_body: None,
        responses: &[
            ResponseSpec {
                status: 200,
                description: "Found",
                schema: Some(SchemaRef {
                    name: "GetOrderResponse",
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
        kind: RouteKind::Passthrough {
            upstream: "order",
            grpc_method: "order.v1.OrderService/ListOrders",
        },
        method: "GET",
        path: "/admin/orders",
        operation_id: "list_orders",
        summary: "List orders with pagination",
        tag: "Orders",
        params: &[
            Param {
                name: "page",
                location: "query",
                required: false,
                schema_type: "integer",
                description: "Page number (default: 1)",
            },
            Param {
                name: "per_page",
                location: "query",
                required: false,
                schema_type: "integer",
                description: "Items per page (default: 20)",
            },
        ],
        request_body: None,
        responses: &[ResponseSpec {
            status: 200,
            description: "Page of orders",
            schema: Some(SchemaRef {
                name: "ListOrdersResponse",
                content_type: "application/json",
            }),
        }],
    },
    ApiRoute {
        kind: RouteKind::Passthrough {
            upstream: "order",
            grpc_method: "order.v1.OrderService/ConfirmOrder",
        },
        method: "PUT",
        path: "/admin/orders/{id}/confirm",
        operation_id: "confirm_order",
        summary: "Confirm a pending order",
        tag: "Orders",
        params: &[Param {
            name: "id",
            location: "path",
            required: true,
            schema_type: "string",
            description: "Order id (UUID)",
        }],
        request_body: None,
        responses: &[ResponseSpec {
            status: 200,
            description: "Confirmed",
            schema: None,
        }],
    },
    ApiRoute {
        kind: RouteKind::Passthrough {
            upstream: "order",
            grpc_method: "order.v1.OrderService/CancelOrder",
        },
        method: "PUT",
        path: "/admin/orders/{id}/cancel",
        operation_id: "cancel_order",
        summary: "Cancel an order",
        tag: "Orders",
        params: &[Param {
            name: "id",
            location: "path",
            required: true,
            schema_type: "string",
            description: "Order id (UUID)",
        }],
        request_body: Some(SchemaRef {
            name: "CancelOrderRequest",
            content_type: "application/json",
        }),
        responses: &[ResponseSpec {
            status: 200,
            description: "Cancelled",
            schema: None,
        }],
    },
    // ── Image upload (presigned URL flow) ─────────────────────────────
    ApiRoute {
        kind: RouteKind::Passthrough {
            upstream: "product",
            grpc_method: "product.v1.ProductService/RequestImageUploadUrl",
        },
        method: "POST",
        path: "/admin/products/{id}/image-upload-url",
        operation_id: "request_image_upload_url",
        summary: "Get a presigned PUT URL for uploading a product image",
        tag: "Products",
        params: &[Param {
            name: "id",
            location: "path",
            required: true,
            schema_type: "string",
            description: "Product id (UUID)",
        }],
        request_body: Some(SchemaRef {
            name: "ImageUploadUrlBody",
            content_type: "application/json",
        }),
        responses: &[
            ResponseSpec {
                status: 200,
                description: "Presigned URL",
                schema: Some(SchemaRef {
                    name: "RequestImageUploadUrlResponse",
                    content_type: "application/json",
                }),
            },
            ResponseSpec {
                status: 404,
                description: "Product not found",
                schema: Some(SchemaRef {
                    name: "ProblemDetail",
                    content_type: "application/problem+json",
                }),
            },
        ],
    },
    ApiRoute {
        kind: RouteKind::Passthrough {
            upstream: "product",
            grpc_method: "product.v1.ProductService/ConfirmImageUpload",
        },
        method: "POST",
        path: "/admin/products/{id}/confirm-image",
        operation_id: "confirm_image_upload",
        summary: "Confirm a product image was successfully uploaded to blob storage",
        tag: "Products",
        params: &[Param {
            name: "id",
            location: "path",
            required: true,
            schema_type: "string",
            description: "Product id (UUID)",
        }],
        request_body: Some(SchemaRef {
            name: "ConfirmImageBody",
            content_type: "application/json",
        }),
        responses: &[
            ResponseSpec {
                status: 204,
                description: "Confirmed",
                schema: None,
            },
            ResponseSpec {
                status: 404,
                description: "Product not found",
                schema: Some(SchemaRef {
                    name: "ProblemDetail",
                    content_type: "application/problem+json",
                }),
            },
            ResponseSpec {
                status: 409,
                description: "Product is inactive",
                schema: Some(SchemaRef {
                    name: "ProblemDetail",
                    content_type: "application/problem+json",
                }),
            },
        ],
    },
];

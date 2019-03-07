# Yet Another Dependency Injector

Very Simple Dependency Injection for Rust

# Usage
Crates has to have `injector.rs`, here example configuration:
```rust 
pub use ::injector::*;
use std::sync::Arc;
use std::env;

dependencies! {
    consts {
        POSTGRES_URL: String = env::var("DATABASE_URL")
            .unwrap_or("postgres://postgres@127.0.0.1:5432/db_name".to_string()),
    },

    services {
        ApiController {
            struct = controller::Controller,
            args = [HttpClient, ModelRepo]
        },

        ModelRepo {
            struct = db::repos::ModelRepo,
            args = [ConnectionPool],
        },

        ConnectionPool {
            struct = Arc<db::ConnectionPool<db::adapter::Postgres>>,
            ctor = |u, l| Arc::new(db::ConnectionPool::new(db::adapter::Postgres::new(u, false), l, 20)),
            args = [POSTGRES_URL]
        },

        HttpClient {
            struct = Arc<http_client::HttpClient>,
            ctor = || Arc::new(http_client::HttpClient::default()),
        }
    }
}

```

And injecting dependencies:
```rust
pub struct ApiController {
  client: dep!(HttpClient),
  model_repo: dep!(ModelRepo),
}

impl ApiController {
  pub fn new(client: dep!(HttpClient), model_repo: dep!(ModelRepo)) -> Self {
    Self {
      client, 
      model_repo,
    }
  }
  
  // other impl methods
}
```

pub mod bind;
pub mod exec;
pub mod explain;
pub mod pool;
pub mod schema;

pub use exec::{execute_batch, execute_query, BatchResult, ExecError, ExecOptions, QueryResult};
pub use explain::{analyze_query, ExplainSummary};
pub use pool::{EngineKind, EnginePool};
pub use schema::{
    describe_table, list_indexes, list_schemas, list_tables, search_objects, ObjectType,
    SchemaObject,
};

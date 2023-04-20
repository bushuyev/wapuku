use std::any::Any;
use std::borrow::Cow;
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufReader, Cursor};
use std::sync::Arc;

use arrow::datatypes::{DataType, Field, Int32Type, Schema, SchemaRef, TimeUnit};
use bytes::Bytes;
use datafusion::datasource::file_format::parquet::*;
use datafusion_common::{Column, OwnedTableReference};
use datafusion_execution::config::SessionConfig;
use datafusion_execution::runtime_env::RuntimeEnv;
use datafusion_expr::{col, Expr, LogicalPlanBuilder, UNNAMED_TABLE};
use datafusion_expr::{AggregateUDF, ScalarUDF, TableSource};
use datafusion_optimizer::analyzer::Analyzer;
use datafusion_optimizer::optimizer::Optimizer;
use datafusion_physical_expr::execution_props::ExecutionProps;
use log::{debug, trace};
use parquet::arrow::parquet_to_arrow_schema;
use parquet::file::footer::{decode_footer, parse_metadata};
use parquet::schema::types::SchemaDescriptor;

struct JsSessionContext {
    state: Arc<JsSessionState>
}

struct JsSessionState {
    analyzer: Analyzer,
    optimizer: Optimizer,
    scalar_functions: HashMap<String, Arc<ScalarUDF>>,
    aggregate_functions: HashMap<String, Arc<AggregateUDF>>,
    config: SessionConfig,
    execution_props: ExecutionProps,
    runtime_env: Arc<RuntimeEnv>,

}


struct JsTableSource {
    table_schema: SchemaRef
}

impl JsSessionState {

}


impl JsTableSource {
    pub fn new(table_schema: SchemaRef) -> Self {
        Self {
            table_schema
        }
    }
}

impl TableSource for JsTableSource {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn schema(&self) -> SchemaRef {
        Arc::clone(&self.table_schema)
    }
}

pub fn parquet_scan(){
    // let x = ListingTable
    let parquet_bytes = include_bytes!("../data/d2_transactions_pi_message.par");

    // let footer_bytes = parquet_bytes[parquet_bytes.len() - 8..parquet_bytes.len()].try_into().unwrap();
    // let metadata_len = decode_footer(&footer_bytes).unwrap();
    // println!("footer_bytes={:?}, metadata_len={:?}", footer_bytes, metadata_len)

    let metadata = parse_metadata(&Bytes::from_static(parquet_bytes)).unwrap();
    let file_metadata = metadata.file_metadata();
    let schema = parquet_to_arrow_schema(
        file_metadata.schema_descr(),
        file_metadata.key_value_metadata(),
    ).unwrap();

    let runtime = RuntimeEnv::default();
    let config  = SessionConfig::default();
    let table_source = Arc::new(JsTableSource::new(Arc::new(schema)));

    let logical_plan_builder = LogicalPlanBuilder::scan(
        UNNAMED_TABLE,
        table_source,
        None
    ).unwrap();

    // let expr = col("PAYMENT_REF").is_not_null();
    let c = Column { relation: Some(OwnedTableReference::Bare { table: Cow::from(UNNAMED_TABLE) }), name: String::from("PAYMENT_REF") };

    let r = logical_plan_builder.filter(Expr::Column(c).is_not_null()).unwrap().build().unwrap();


    debug!("parquet_scan: metadata={:?}", r);

    // let parquet_schema= SchemaDescriptor::new(ptr);
    // 
    // let schema = parquet_to_arrow_schema(
    //     &parquet_schema,
    //     Some(&vec![])
    // );

    // let pf = ParquetFormat::default();
    // fetch_parquet_metadata()

    // let pfr = DefaultParquetFileReaderFactory
}
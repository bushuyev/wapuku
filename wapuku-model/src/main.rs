use std::any::Any;
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufReader, Cursor};
use std::sync::Arc;

use datafusion_execution::config::SessionConfig;
use datafusion_execution::runtime_env::RuntimeEnv;
use datafusion_expr::{AggregateUDF, ScalarUDF, TableSource};
use datafusion_optimizer::analyzer::Analyzer;
use datafusion_optimizer::optimizer::Optimizer;
use datafusion_physical_expr::execution_props::ExecutionProps;
use arrow::datatypes::{DataType, Field, Int32Type, Schema, SchemaRef, TimeUnit};

pub mod model;

fn main() {
// 
//     let f = File::open("../www/data/wapuku.obj").unwrap();
//     // let f = File::open("/home/bu/dvl/rust/learn-wgpu/code/beginner/tutorial9-models/res/cube.obj").unwrap();
// 
//     let mut obj_reader = BufReader::new(f);
// 
//     let (mesh, mat_r) = tobj::load_obj_buf(
//         &mut obj_reader,
//         &tobj::LoadOptions {
//             triangulate: true,
//             single_index: true,
//             ..Default::default()
//         },
//         |p| {
// 
//             println!("p={:?}", p);
//             let mat_text = "";
//             
//             // println!("mat_text={}", mat_text);
// 
//             let f = File::open("../www/data/wapuku.mtl").unwrap();
//             // let f = File::open("/home/bu/dvl/rust/learn-wgpu/code/beginner/tutorial9-models/res/cube.mtl").unwrap();
// 
//             let mut mtl_reader = BufReader::new(f);
//             
//             tobj::load_mtl_buf(&mut mtl_reader)
//         }
//     ).unwrap();
//     
//     println!("model={:?}", mesh);
//     println!("mat={:?}", mat_r.unwrap());
// 
}

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



#[cfg(test)]
mod tests {
    use std::borrow::Cow;
    use std::sync::Arc;
    use datafusion_execution::runtime_env::RuntimeEnv;
    use datafusion_execution::config::SessionConfig;
    use datafusion_expr::{col, Expr, LogicalPlanBuilder, UNNAMED_TABLE};
    use crate::{JsTableSource, model};
    use datafusion::datasource::file_format::parquet::*;
    use parquet::arrow::parquet_to_arrow_schema;
    use parquet::file::footer::{decode_footer, parse_metadata};
    use parquet::schema::types::SchemaDescriptor;
    use bytes::Bytes;
    use datafusion_common::{Column, OwnedTableReference};

    #[test]
    fn test_1() {
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
        let c = Column { relation: Some(OwnedTableReference::Bare { table: Cow::from("?table?") }), name: String::from("PAYMENT_REF") };
        
        let r = logical_plan_builder.filter(Expr::Column(c).is_not_null()).unwrap().build().unwrap();

        
        println!("metadata={:?}", r)
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

}

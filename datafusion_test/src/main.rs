use std::borrow::Cow;
use datafusion::prelude::{col, Expr, ParquetReadOptions, SessionContext};
use datafusion::common::{Column, OwnedTableReference};
use datafusion::logical_expr::UNNAMED_TABLE;

#[tokio::main]
async fn main() {
    let ctx = SessionContext::new();
    let df = ctx.read_parquet("../wapuku-model/data/d2_transactions_pi_message.par", ParquetReadOptions {
        file_extension: "par",
        table_partition_cols: vec![],
        parquet_pruning: None,
        skip_metadata: None,
    }).await.unwrap();

    // println!("schema={:?}", df.schema());
    println!("");
    
    // let c = Column::from_qualified_name("PAYMENT_REF");
    
    let c = Column { relation: Some(OwnedTableReference::Bare { table: Cow::from(UNNAMED_TABLE) }), name: String::from("PAYMENT_REF") };
    
    
    let df = df.filter(Expr::Column(c).is_not_null()).unwrap().limit(0, Some(100)).unwrap();
    
    // df.count();

    println!("df.count()={:?}", df.count().await);
}

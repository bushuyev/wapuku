use std::any::Any;
use std::borrow::Cow;
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufReader, Cursor};
use std::sync::Arc;

use polars::prelude::*;
use polars::time::*;
use polars::time::{Duration};
// use polars::time::windows::*;
// use polars::prelude::windows::group_by::*;
use polars::io::parquet::*;

use polars::lazy::*;
// use arrow::datatypes::{DataType, Field, Int32Type, Schema, SchemaRef, TimeUnit};
use bytes::Bytes;

use log::{debug, trace};
use polars::prelude::Expr::Columns;
// use parquet::arrow::parquet_to_arrow_schema;
// use parquet::file::footer::{decode_footer, parse_metadata};
// use parquet::schema::types::SchemaDescriptor;
use smartstring::alias::String as SmartString;

pub fn parquet_scan() {
    let parquet_bytes = include_bytes!("../../wapuku-model/data/s1_transactions_pi_message.par");

    let mut buff = Cursor::new(parquet_bytes);

    let mut df = ParquetReader::new(buff)
        .finish().unwrap()
        .lazy()
        .groupby([col("PAYMENTSTATUS")])
        .agg([count()])
        .sort("count", SortOptions {
            descending: true,
            nulls_last: true,
            multithreaded: true,
        },)
        .collect()
        .unwrap();

    debug!("parquet_scan: height={:?}", df.height());

}

pub fn simp_1() {
    let df = df!(
        "field_1" => &[1,       1,      2,      2,    2],
        "field_2" => &["a",     "b",    "c",    "d",  "e"]
    ).unwrap();

    let df = df.groupby(["field_1"]).unwrap().select(["field_2"]).groups().unwrap();

    debug!("parquet_scan: df={:?}", df);
}


pub fn group_by() {
    let mut df = df!(
        "field_1" => &[10,      20,     30,     40,   41,    50,     60,     70,     80,     90], 
        "field_2" => &[1,       1,      1,      1,    3,     2,      2,      2,      2,      2],
        "field_3" => &["a",     "b",    "c",    "d",  "dd",   "e",    "f",    "g",    "h",    "ii"]
    ).unwrap();

    // let min_max = df.clone().lazy().select([
    //     col("field_1").min().alias("field_1_min"),
    //     col("field_1").max().alias("field_1_max")
    // ]).collect();

    // let df = df.clone()
    //     .lazy()
    // .groupby([col("field_2").lt(20)])
    // .groupby([10])
    // .agg([sum("field_2")])
    // .collect();
    // debug!("parquet_scan: df={:?} min_max={:?}", df, min_max);

    let field2_grouped = df.clone()
        .lazy()
        .select(&[col("field_2")])
        .sort("field_2", Default::default())
        .groupby_dynamic([],
                         DynamicGroupOptions {
                             index_column: "field_2".into(),
                             every: Duration::new(3),
                             period: Duration::new(3),
                             offset: Duration::new(0),
                             truncate: true,
                             include_boundaries: false,
                             closed_window: ClosedWindow::Left,
                             start_by: Default::default(),
                         }
        ).agg([col("field_2").alias("field_2_groupped")]).explode(["field_2_groupped"]).collect().unwrap();


    debug!("parquet_scan: df={:?}", field2_grouped);
    let df = df.with_column(field2_grouped.column("field_2_groupped").unwrap().clone()).unwrap();

    let df = df.clone()//.vstack(&field2_grouped).unwrap()
        .lazy()
        // .left_join(field2_grouped, col("field_2"), col("field_2_groupped"))
        .groupby_dynamic(
            // [Expr::Columns(vec![ String::from("field_2")])], 
            [col("field_2_groupped")],
            // [], 
            DynamicGroupOptions {
                index_column: "field_1".into(),
                every: Duration::new(20),
                period: Duration::new(20),
                offset: Duration::new(0),
                truncate: true,
                include_boundaries: true,
                closed_window: ClosedWindow::Left,
                start_by: Default::default(),
            }
        )
        // .agg([sum("field_2")])
        .agg([col("field_3"), col("field_1").alias("_field_1")])
        .collect();

    debug!("parquet_scan: df={:?}", df);

    // debug!("parquet_scan: time_key={:?}, keys={:?}, groups={:?}", time_key, keys, groups);

}
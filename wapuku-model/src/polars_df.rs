use std::any::Any;
use std::borrow::Cow;
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufReader, Cursor};
use std::marker::PhantomData;
use std::sync::Arc;

use bytes::Bytes;
use log::{debug, trace};
// use polars::time::windows::*;
// use polars::prelude::windows::group_by::*;
use polars::io::parquet::*;
use polars::lazy::*;
use polars::prelude::*;
use polars::prelude::Expr::Columns;
use polars::time::*;
use polars::time::Duration;
// use parquet::arrow::parquet_to_arrow_schema;
// use parquet::file::footer::{decode_footer, parse_metadata};
// use parquet::schema::types::SchemaDescriptor;
use smartstring::alias::String as SmartString;

use crate::model::*;
use crate::data_type::WapukuDataType;

impl From<PolarsError> for WapukuError {
    fn from(value: PolarsError) -> Self {
        WapukuError::DataFrame{msg: value.to_string()}
    }
}



pub struct PolarsData<'a> {
    property_sets: Vec<SimplePropertiesSet>,
    pd: PhantomData<&'a SimplePropertiesSet>,
}

impl <'a> PolarsData<'a> {

    pub fn new() -> Self {
        parquet_scan();
        
        Self {

            property_sets: vec![SimplePropertiesSet::new(
                vec![
                    DataProperty::new(WapukuDataType::Numeric, "property_1"),
                    DataProperty::new(WapukuDataType::Numeric, "property_2"),
                    DataProperty::new(WapukuDataType::Numeric, "property_3"),
                ],
                "item_1",
            )], 
            pd: PhantomData
        }
    }
}

impl <'a> Data<'a> for PolarsData<'a> {
    type DataGroupType = SimpleDataGroup<'a>;

    fn all_sets(&self) -> Vec<&dyn PropertiesSet> {
        self.property_sets.iter().fold(vec![], |mut props, p| {
            props.push(p);

            props
        })
    }

    fn group_by_1(&self, property_range: PropertyRange<'a>) -> GroupsVec<SimpleDataGroup<'a>> {

        GroupsVec::new(property_range.property(), vec![
            SimpleDataGroup::new(10, vec![], DataBounds::X(property_range.to_range(Some(0.0),Some(10.0))))
        ])
    }

    fn group_by_2(&self, property_x: PropertyRange<'a>, property_y: PropertyRange<'a>) -> GroupsGrid<Self::DataGroupType> {

        GroupsGrid::new(
            property_x.property(),
            property_y.property(),
            vec![
                (0..10).map(|i|/*Box::<dyn DataGroup>::from(Box::new(*/
                    SimpleDataGroup::new(10, vec![],
                                         DataBounds::XY(
                                             property_x.to_range(Some(i as f64 * 10.0), Some(i as f64 * 10.0 + 10.0)),
                                             property_y.to_range(Some(i as f64 * 10.0), Some(i as f64 * 10.0 + 10.0))
                                         )
                    )
                ).collect()
            ]
        )
    }

}


pub fn parquet_scan() {
    // let parquet_bytes = include_bytes!("../../wapuku-model/data/s1_transactions_pi_message.par");
    let parquet_bytes = include_bytes!("../../wapuku-model/data/d2_transactions_pi_message.par");
    
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



pub(crate) fn group_by(df:&DataFrame, main_group_by_field: &str, second_group_by_field: &str) -> WapukuResult<DataFrame> {
    let field_2_groupped_str = format!("{}_groupped", second_group_by_field);
    let field_2_left_str = format!("{}_left", main_group_by_field);
    
    let field2_grouped = df.clone()
        .lazy()
        .select(&[col(second_group_by_field).alias(field_2_left_str.as_str())])
        .sort(field_2_left_str.as_str(), Default::default())
        .groupby_dynamic([], 
 DynamicGroupOptions {
            index_column: field_2_left_str.as_str().into(),
            every: Duration::new(2),
            period: Duration::new(2),
            offset: Duration::new(0),
            truncate: true,
            include_boundaries: true,
            closed_window: ClosedWindow::Left,
            start_by: Default::default(),
        }
    ).agg([col(field_2_left_str.as_str()).alias(field_2_groupped_str.as_str())]).explode([field_2_groupped_str.as_str()]).collect()?;


    debug!("field2_grouped: field2_grouped={:?}", field2_grouped);
    let mut df = df.sort([second_group_by_field], false)?;
    let df = df.with_column(field2_grouped.column(field_2_left_str.as_str()).unwrap().clone())?;

    let mut df = df.sort([main_group_by_field], false)?;
    debug!("field2_grouped: df={:?}", df);
    
    let df = df.clone()
        .lazy()
        .groupby_dynamic(
            [col(field_2_left_str.as_str())],
            DynamicGroupOptions {
                index_column: main_group_by_field.into(),
                every: Duration::new(20),
                period: Duration::new(20),
                offset: Duration::new(0),
                truncate: true,
                include_boundaries: false,
                closed_window: ClosedWindow::Left,
                start_by: Default::default(),
         }
        )
        .agg([col("field_3").alias("field_3_value"), col(main_group_by_field).alias("field_1_value")])
        .collect()?;
    
    debug!("parquet_scan: df={:?}", df);

    Ok(df)
}

#[cfg(test)]
mod tests {
    use log::debug;
    use polars::datatypes::AnyValue::List;
    use polars::df;
    use polars::prelude::*;

    use crate::polars_df::group_by;
    use crate::tests::init_log;

    #[ctor::ctor]
    fn init() {
        std::env::set_var("POLARS_FMT_MAX_ROWS", "100");
        std::env::set_var("FMT_MAX_COLS", "1000");
        
        init_log();
    }

    #[test]
    fn test_group_by() {
        

        let mut df = df!(
            "field_1" => &[10,      20,     30,     40,   41,    50,     60,     70,     80,     90], 
            "field_2" => &[1,       1,      1,      1,    3,     2,      2,      2,      2,      2],
            "field_3" => &["a",     "b",    "c",    "d",  "dd",  "e",   "f",    "g",    "h",    "ii"]
        ).unwrap();

        /*
        _____|_0-20_|_20-40_|_40-60_|_60-80_|_80-100_|
         0-2 |  a      bc      d
         2-4 |                dd e    fg      h ii
         4-6 |
         6-8 |
         8-10|
        
         */
        let df = group_by(&df, "field_1", "field_2").expect("df");
        
        debug!("df={:?}", df);
       
        assert_eq!(*df.column("field_3_value").expect("field_3_valuee"),
        Series::new("field_3_value", [
            List(Series::new("", ["a"])),
            List(Series::new("", ["b", "c"])),
            List(Series::new("", ["d"])),
            List(Series::new("", ["dd", "e"])),
            List(Series::new("", ["f", "g"])),
            List(Series::new("", ["h", "ii"])),
        ]));
     
        
    }
    
    #[test]
    fn test_simp(){
        let df = df!(
        "field_1" => &[1,       1,      2,      2,    2],
        "field_2" => &["a",     "b",    "c",    "d",  "e"]
    ).unwrap();

        let group_by = df.groupby(["field_1"]).unwrap();
        let by = group_by.select(["field_2"]);
        let df = by.groups().unwrap();

        debug!("parquet_scan: df={:?}", df);
        

    }

}
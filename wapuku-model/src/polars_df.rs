use std::collections::HashSet;
use std::io::{Cursor, Read};

use log::debug;
use polars::io::parquet::*;
use polars::prelude::*;
use polars::prelude::StartBy::WindowBound;
use polars::time::Duration;

use crate::data_type::WapukuDataType;
use crate::model::*;
use ::zip::*;
use ::zip::result::*;

impl From<PolarsError> for WapukuError {
    fn from(value: PolarsError) -> Self {
        WapukuError::DataFrame{msg: value.to_string()}
    }
}


#[derive(Debug)]
pub struct PolarsData {
    df:DataFrame,
    property_sets: Vec<SimplePropertiesSet>,
    name:String
}

impl From<ZipError> for WapukuError {
    fn from(value: ZipError) -> Self {
        WapukuError::DataLoad{msg: value.to_string()}
    }
}

impl PolarsData {

    pub fn new(df:DataFrame, name:String) -> Self {//TODO move to add df
        // parquet_scan();

        let properties = df.schema().iter_fields().map(|f| DataProperty::new(WapukuDataType::Numeric, f.name)).collect();
        Self {
            df,
            property_sets: vec![SimplePropertiesSet::new(
                properties,
                "item_1",
            )],
            name
        }
    }
}

impl Data for PolarsData {



    fn load(data:Box<Vec<u8>>, name: Box<String>) -> Result<Vec<Self>, WapukuError> {
        if name.ends_with("csv") {
            load_csv(data).map(|d|vec![PolarsData::new(d, *name)])

        } else if name.ends_with("parquet") {
            load_parquet(data).map(|d|vec![PolarsData::new(d, *name)])

        } else if name.ends_with("zip") {
            load_zip(data).map(|d_vec|d_vec.into_iter().map(|(df, entry_name)|PolarsData::new(df, format!("{}/{}", name, entry_name))).collect())

        } else {
            Err(WapukuError::General{ msg: String::from("I can load only csv or parquet files")})
        }

    }

    fn name(&self) -> String {
        self.name.clone()
    }


    fn all_sets(&self) -> Vec<&dyn PropertiesSet> {
        self.property_sets.iter().fold(vec![], |mut props, p| {
            props.push(p);

            props
        })
    }

    fn all_properties(&self) -> HashSet<&dyn Property> {
        self.property_sets.iter().flat_map(|property_set|property_set.properties().into_iter()).collect()
    }



    fn build_grid(&self, property_x: PropertyRange, property_y: PropertyRange, x_n: u8, y_n: u8, group_volume_property: &str) -> GroupsGrid {
        let property_x_name = property_x.property().name().as_str();
        let property_y_name = property_y.property().name().as_str();

        let properties_df = self.df.select([property_x_name, property_y_name]).unwrap();
        let min_df = properties_df.min();
        let max_df = properties_df.max();

        let min_x =  property_x.min().unwrap_or(min_df.column(property_x_name).unwrap().get(0).unwrap().try_extract::<f32>().unwrap() as i64) as f32;
        let max_x = property_x.max().unwrap_or(max_df.column(property_x_name).unwrap().get(0).unwrap().try_extract::<f32>().unwrap() as i64) as f32;


        let min_y =  property_y.min().unwrap_or(min_df.column(property_y_name).unwrap().get(0).unwrap().try_extract::<f32>().unwrap() as i64) as f32;
        let max_y = property_y.max().unwrap_or(max_df.column(property_y_name).unwrap().get(0).unwrap().try_extract::<f32>().unwrap() as i64) as f32;

        let property_x_step = (( (max_x - min_x) / x_n as f32).ceil()) as i64;

        let property_y_step = (((max_y - min_y)  / y_n as f32).ceil()) as i64;

        // debug!("wapuku: min_df={:?} max_df={:?}", min_df, max_df);
        debug!("wapuku: min/max_x={:?}, min/max_y={:?} property_x_step={:?}, property_y_step={:?}", (min_x, max_x), (min_y, max_y), property_x_step, property_y_step);

        let df = if max_x == min_x {

            group_by_1(&self.df,
               property_y_name, property_y_step,
               [
                   col(group_volume_property).count().alias("volume"),
               ], min_y as i64 - property_y_step
            ).unwrap()

        } else if max_y == min_y {

            group_by_1(&self.df,
               property_x_name, property_x_step,
               [
                   col(group_volume_property).count().alias("volume"),
               ], min_x as i64 - property_x_step
            ).unwrap()

        } else {

            group_by_2(&self.df,
                property_x_name, property_x_step,
                property_y_name, property_y_step,
                [
                    col(group_volume_property).count().alias("volume"),
                ], min_x as i64 - property_x_step, min_y as i64 - property_y_step
            ).unwrap()
        };



        debug!("1. wapuku: df={:?}", df);

        // // https://stackoverflow.com/questions/72440403/iterate-over-rows-polars-rust df is small here, should be ok
        // df.as_single_chunk_par();
        // let mut iters = df.columns(["primary_field_group", "secondary_group_field", "volume"]).expect("grouping columns")
        //     .iter().map(|s| s.iter()).collect::<Vec<_>>();
        //


        let mut data_vec : Vec<Vec<Option<Box<dyn DataGroup>>>> = (0..y_n).map(|_y| (0..x_n).map(|_x| None).collect()).collect();



        (0..y_n as usize).for_each(|y|{
            (0..x_n as usize).for_each(|x| {
                let x_0 = min_x as i64 + x as i64 * property_x_step ;
                 let x_1 = min_x as i64 + x as i64 * property_x_step + property_x_step;
                 let y_0 = min_y as i64 + y as i64 * property_y_step;
                 let y_1 = min_y as i64 + y as i64 * property_y_step + property_y_step;

                let count = df.clone().lazy()
                     .filter(
                         if max_x == min_x {
                             col("group_by_field").gt_eq(y_0).and(col("group_by_field").lt(y_1))
                         } else if max_y == min_y {
                             col("group_by_field").gt_eq(x_0).and(col("group_by_field").lt(x_1))
                         } else {
                             col("primary_field_group").gt_eq(x_0).and(col("primary_field_group").lt(x_1)
                             .and(
                                 col("secondary_group_field").gt_eq(y_0).and(col("secondary_group_field").lt(y_1))
                             ))
                         }

                     )
                     .select([col("volume")]).collect().unwrap();

                let v = count.get(0).and_then(|v| v.get(0).map(|v| v.try_extract::<u32>().unwrap())).unwrap_or(0) as usize;


                debug!("wapuku: data_vec: (x, y)={:?}, (x_0, x_1)={:?}, (y_0, y_1)={:?}, v={:?}", (x, y), (x_0, x_1), (y_0, y_1), v);

                if v > 0 {
                    let group_box = Box::<dyn DataGroup>::from(Box::new(SimpleDataGroup::new(
                        v,
                        vec![],
                        DataBounds::XY(property_x.to_range(Some(x_0), Some(x_1)), property_y.to_range(Some(y_0), Some(y_1))),
                    )));

                    data_vec[y][x].replace(group_box);
                }

            });
        });


        GroupsGrid::new(
            property_x.property().clone_to_box(),
            property_y.property().clone_to_box(),
            data_vec
        )
    }

    fn build_summary(&self) -> Summary {

        //TODO
        // self.df.get_columns().len() < 20

        let desc = self.df.describe(None).unwrap();
        // self.df.get_column_names().into_iter().zip(desc.iter()).for_each((|(name, column)|{
        debug!("head={:?}", desc.head(None));
        debug!("get={:?} desc.get_columns().len()={}", desc.get(0), desc.get_columns().len());
        // desc.get_columns().iter().map(|c|c.)

        Summary::new (desc.get_columns().into_iter().enumerate().skip(1).map(|(i, c)|{

            if i%1000 == 0 {
                debug!("column={:?} type={:?} mean={:?}", c.name(), c.dtype(), desc.get(2).map(|row|row.get(i).map(|v|format!("{}", v))));
            }

            let data_type = map_to_wapuku(c.dtype());
            match data_type {

                WapukuDataType::Numeric { .. } => {
                    ColumnSummary::new(
                        String::from(c.name()),
                        ColumnSummaryType::Numeric{data:NumericColumnSummary::new(
                            desc.get(4).and_then(|row|row.get(i).map(|v|format!("{}", v))).unwrap_or(String::from("n/a")),
                            desc.get(2).and_then(|row|row.get(i).map(|v|format!("{}", v))).unwrap_or(String::from("n/a")),
                            desc.get(8).and_then(|row|row.get(i).map(|v|format!("{}", v))).unwrap_or(String::from("n/a")),
                        )}
                    )
                }

                WapukuDataType::String => {

                    let unique_values = self.df.column(c.name())
                        .and_then(|v|v.unique())
                        .map(|u|
                            u.rechunk().iter()
                            .take(3)
                            .map(|v|{
                                ToString::to_string(&v)
                            })
                            .collect::<Vec<String>>().join(", ")
                        ).unwrap_or(String::from("N/A"));


                    debug!("unique_values={:?}", unique_values);

                    ColumnSummary::new(
                        String::from(c.name()),
                        ColumnSummaryType::String { data:StringColumnSummary::new(unique_values)},
                    )
                }

                WapukuDataType::Boolean => {
                    ColumnSummary::new(
                        String::from(c.name()),
                        ColumnSummaryType::Boolean,
                    )
                }
            }

        }).collect())
        // desc.iter().for_each((|column|{
        //     // println!("name={} column={:?}", name, column);
        //     // let c_v = column.iter().map(|v|String::from(v.get_str().unwrap_or("zzz"))).collect::<Vec<String>>();
        //     column.iter().for_each(|v|{
        //         println!("{:?}", v);
        //     });
        //     // println!("column={:?} min={:?}",  column.name(), c_v[2]);
        //     println!("column={:?}",  column.name());
        // }));
        // println!("head={}", desc.unwrap().head(None));
        // println!("desc={:?}", desc);
        // Summary::new (self.df.get_column_names().into_iter().map(|c|ColumnSummary::new(String::from(c), 0., 1., 2.)).collect())
    }
}

fn map_to_wapuku(d_type: &DataType) -> WapukuDataType {
    //TODO
    match d_type {
        DataType::Utf8=> WapukuDataType::String,
        DataType::Float64 => WapukuDataType::Numeric,
        DataType::Boolean => WapukuDataType::Boolean,
        _ => WapukuDataType::String
    }
}

pub fn fake_df() -> DataFrame {
    // df!(
    //     "property_1" => &(0..10_000_000).into_iter().map(|i|i / 10).collect::<Vec<i64>>(), // 10 X 0, 10 X 1 ...
    //     "property_2" => &(0..10_000_000).into_iter().map(|i|i - (i/10)*10 ).collect::<Vec<i64>>(), //
    //     "property_3" => &(0..10_000_000).into_iter().map(|i|i).collect::<Vec<i32>>(),
    // ).unwrap()
    // df!(
    //     "property_1" => &[1,   2,   3,   1,   2,   3,   1,   2,   3,],
    //     "property_2" => &[10,  10,  10,  20,  20,  20,  30,  30,  30,],
    //     "property_3" => &[11,  12,  13,  21,  22,  23,  31,  32,  33,]
    // ).unwrap()

    df!(
       "property_1" => &(0..10_000).into_iter().map(|i|i / 100).collect::<Vec<i64>>(), // 10 X 0, 10 X 1 ...
       "property_2" => &(0..10_000).into_iter().map(|i|i - (i/100)*100 ).collect::<Vec<i64>>(), //
       "property_3" => &(0..10_000).into_iter().map(|i|i).collect::<Vec<i32>>(),
    ).unwrap()
}

pub fn load_zip(data: Box<Vec<u8>>) -> Result<Vec<(DataFrame, String)>, WapukuError> {
    let mut archive = ZipArchive::new(Cursor::new(data.as_slice()))?;
    archive.file_names().map(|s| String::from(s)).collect::<Vec<String>>().into_iter().map(|file| {

        let mut bytes = Vec::new();
        archive.by_name(file.as_str())?.read_to_end(&mut bytes);

        if file.ends_with("csv") {
            load_csv(Box::new(bytes)).map(|df|(df, file))
        } else if file.ends_with("parquet") {
            load_parquet(Box::new(bytes)).map(|df|(df, file))
        } else {
            Err(WapukuError::DataLoad {msg: format!("Unexepcted file ending {}", file)})
        }
    }).collect::<Result<Vec<(DataFrame, String)>, WapukuError>>()
}


pub fn load_csv(csv_bytes:Box<Vec<u8>>) -> Result<DataFrame, WapukuError> {
    CsvReader::new(Cursor::new(csv_bytes.as_slice())).finish().map_err(|e|e.into())
}

pub fn load_parquet(parquet_bytes:Box<Vec<u8>>) -> Result<DataFrame, WapukuError> {
    ParquetReader::new(Cursor::new(parquet_bytes.as_slice())).finish().map_err(|e|e.into())
}

pub(crate) fn group_by_1<E: AsRef<[Expr]>>(df:&DataFrame, group_by_field: &str,  step: i64, aggregations: E, offset: i64) -> WapukuResult<DataFrame> {

    let df = df.sort([group_by_field], vec![true], false)?;

    let df = df.clone()
        .lazy()
        .sort(group_by_field, SortOptions::default())
        .groupby_dynamic(
            col(group_by_field.into()),
            [],
            DynamicGroupOptions {
                index_column: group_by_field.into(),
                every: Duration::new(step),
                period: Duration::new(step),
                offset: Duration::new(offset),
                truncate: false,
                include_boundaries: true,
                closed_window: ClosedWindow::Left,
                start_by: WindowBound,
                check_sorted: true
            }
        )
        .agg(aggregations)
        // .agg([
        //     col("property_3").count().alias("volume")
        // ])
        .collect()?;

    let mut df = df.sort([group_by_field], vec![true], false)?;

    df.rename(group_by_field, "group_by_field").expect("rename group_by_field");
    // let df = df.clone().lazy().with_column(lit(1).alias("primary_field_group")).collect()?;
    // df.rename(group_by_field, "secondary_group_field");

    debug!("wapuku: df grouped={:?}", df);

    Ok(df)
}


pub(crate) fn group_by_2<E: AsRef<[Expr]>>(df:&DataFrame, primary_group_by_field: &str, primary_step: i64, secondary_group_by_field: &str, secondary_step: i64, aggregations: E, primary_offset: i64, secondary_offset: i64) -> WapukuResult<DataFrame> {
    let primary_field_value = "primary_field_value";
    let primary_field_group = "primary_field_group"; //expanded value of the field in the second column group, to be joined back to main frame

    let primary_field_grouped_and_expanded = df.clone()
        .lazy()
        .select(&[
            col(primary_group_by_field).alias(primary_field_group)
        ])
        .sort(primary_field_group, Default::default())
        .groupby_dynamic(
            col(primary_field_group.into()),
            [],
 DynamicGroupOptions {
            index_column: primary_field_group.into(),
            every: Duration::new(primary_step),
            period: Duration::new(primary_step),
            offset: Duration::new(primary_offset),
            truncate: false,
            include_boundaries: true,
            closed_window: ClosedWindow::Left,
            start_by: WindowBound,
            check_sorted: true
        }
    ).agg([
        col(primary_field_group).alias(primary_field_value)
    ])/*.with_row_count("primary_index", None)*/
        .explode([primary_field_value]).collect()?;//primary_field_in_group not used, for debug


    debug!("wapuku: primary_field_grouped_and_expanded={:?}", primary_field_grouped_and_expanded);
    let mut df = df.sort([primary_group_by_field], vec![true], false)?;

    let df = df
        .with_column(primary_field_grouped_and_expanded.column(primary_field_group).unwrap().clone())?
        // .with_column(primary_field_grouped_and_expanded.column("primary_index").unwrap().clone())?
        // .with_column({
        //     let mut series = primary_field_grouped_and_expanded.column("_lower_boundary").unwrap().clone();
        //     series.rename("primary_lower_boundary");
        //     series
        // })?
    ;
    let df = df.sort([secondary_group_by_field], vec![true], false)?;

    // let mut df = df.left_join(&primary_field_grouped_and_expanded, [primary_group_by_field], ["primary_field_value"] )?;


    debug!("2. wapuku: df={:?}", df);

    let mut df = df.clone()
        .lazy()
        .sort(secondary_group_by_field, Default::default())
        .groupby_dynamic(
            col(secondary_group_by_field.into()),
            [col(primary_field_group)],
            DynamicGroupOptions {
                index_column: secondary_group_by_field.into(),
                every: Duration::new(secondary_step),
                period: Duration::new(secondary_step),
                offset: Duration::new(secondary_offset),
                truncate: false,
                include_boundaries: true,
                closed_window: ClosedWindow::Left,
                start_by: WindowBound,
                check_sorted: true
         }
        )
        .agg(aggregations)
        // .agg([
        //     col("property_3").count().alias("volume")
        // ])
        .collect()?;

    df.rename(secondary_group_by_field, "secondary_group_field").expect("rename secondary_group_field");

    debug!("wapuku: df grouped={:?}", df);

    Ok(df)
}

#[cfg(test)]
mod tests {
    use std::any::{Any, TypeId};
    use std::collections::HashSet;
    use crate::tests::init_log;

    use log::debug;
    use polars::datatypes::AnyValue::List;
    use polars::df;
    use polars::prelude::*;

    use crate::data_type::{WapukuDataType};
    use crate::model::{ColumnSummaryType, Data, DataGroup, DataProperty, GroupsGrid, NumericColumnSummary, Property, PropertyRange, Summary};
    use crate::polars_df::{group_by_2, PolarsData};

    #[ctor::ctor]
    fn init() {
        std::env::set_var("POLARS_FMT_MAX_ROWS", "100");
        std::env::set_var("FMT_MAX_COLS", "1000");

        init_log();
    }

    #[test]
    fn test_build_summary_ints(){
        let mut df = df!(
            "property_1" => &[1,   2,   3,   1,   2,   3,   1,   2,   3,],
            "property_2" => &[10,  10,  10,  20,  20,  20,  30,  30,  30,],
            "property_3" => &[11,  12,  13,  21,  22,  23,  31,  32,  33,]
        ).unwrap();
        let mut data = PolarsData::new(df, String::from("test"));

        let summary = data.build_summary();

        check_numeric_column(&summary, 0, "1.0");
        check_numeric_column(&summary, 1, "10.0");
        check_numeric_column(&summary, 2, "11.0");
        // assert_eq!(summary.numeric_columns(0).unwrap().min(), "1.0");
        // assert_eq!(summary.numeric_columns(1).unwrap().min(), "10.0");
        // assert_eq!(summary.numeric_columns(2).unwrap().min(), "11.0");
    }

    #[test]
    fn test_build_summary_str(){
        let mut df = df!(
            "property_1" => &[1i32,   2i32,    3i32,    1i32,       2i32,    3i32,    1i32,    2i32,   3i32],
            "property_2" => &[1f32,   2f32,    3f32,  1f32,    2f32,    3f32,    1f32,    2f32,   3f32],
            "property_3" => &["A",  "B",  "C",  "D",  "E",  "F",  "G",  "H", "I"]
        ).unwrap();
        let mut data = PolarsData::new(df, String::from("test"));

        let summary = data.build_summary();

        // for c in summary.columns() {
        //     println!("type=1={:?}",  TypeId::of::<u8>() == c.type_id());
        //     println!("type=1={:?}",  TypeId::of::<f32>() == c.type_id());
        //     println!("type=1={:?}",  TypeId::of::<String>() == c.type_id());
        // }

        check_numeric_column(&summary, 0, "1.0");
        // check_numeric_column(&summary, 1, "1.0");




        // assert_eq!(summary.columns()[1].min(), "10.0");
        // assert_eq!(summary.columns()[2].min(), "11.0");
    }

    fn check_numeric_column(summary: &Summary, i: usize, value: &str) {
        if let ColumnSummaryType::Numeric { data } = summary.columns()[i].dtype() {
            assert_eq!(data.min(), value);
        } else {
            panic!("column {} is not numeric", i)
        }
    }

    #[test]
    fn test_polars_data_1x3x3(){
        /**
         property_1     1   2   3    - X
         property_2
            1           11  12  13 
            2           21  22  23
            3           31  32  33

            Y
         
        **/

        let mut df = df!(
            "property_1" => &[1,   2,   3,   1,   2,   3,   1,   2,   3,], 
            "property_2" => &[10,  10,  10,  20,  20,  20,  30,  30,  30,],
            "property_3" => &[11,  12,  13,  21,  22,  23,  31,  32,  33,] 
        ).unwrap();

        debug!("wapuku: df: {:?}", df);


        // let mut grid = x_property_1_y_property_2_to_3_x_3_data(df, (Some(1i64), Some(4i64)), (Some(10i64), Some(31i64)));
        let grid = x_property_1_y_property_2_to_3_x_3_data(df, (None, None), (None, None));

        debug!("wapuku: grid: {:?}", grid);
        
        // let data = grid.data();

        
        assert_eq!(grid.group_at(0, 0).unwrap().volume(), 1);
        assert_eq!(grid.group_at(0, 1).unwrap().volume(), 1);
        assert_eq!(grid.group_at(0, 2).unwrap().volume(), 1);

        assert_eq!(grid.group_at(1, 0).unwrap().volume(), 1);
        assert_eq!(grid.group_at(1, 1).unwrap().volume(), 1);
        assert_eq!(grid.group_at(1, 2).unwrap().volume(), 1);

        assert_eq!(grid.group_at(2, 0).unwrap().volume(), 1);
        assert_eq!(grid.group_at(2, 1).unwrap().volume(), 1);
        assert_eq!(grid.group_at(2, 2).unwrap().volume(), 1);

    }

    #[test]
    fn test_polars_data_1x1x3(){
        /**
         property_1     1   2   3    - X
         property_2
            1           11  
            2           21  
            3           31  

            Y

        **/

        let mut df = df!(
            "property_1" => &[1,   1,   1,  ], 
            "property_2" => &[10,  20,  30, ],
            "property_3" => &[11,  21,  31, ] 
        ).unwrap();

        debug!("wapuku: df: {:?}", df);


        let grid = x_property_1_y_property_2_to_3_x_3_data(df, (Some(1_i64), Some(1_i64)), (Some(10i64), Some(31i64)));
        // let mut grid = x_property_1_y_property_2_to_3_x_3_data(df, (None, None), (None, None));

        debug!("wapuku: grid: {:?}", grid);

        // let data = grid.data();


        assert_eq!(grid.group_at(0, 0).unwrap().volume(), 1);
        assert_eq!(grid.group_at(0, 1).unwrap().volume(), 1);
        assert_eq!(grid.group_at(0, 2).unwrap().volume(), 1);

        assert_eq!(grid.group_at(1, 0).unwrap().volume(), 1);
        assert_eq!(grid.group_at(1, 1).unwrap().volume(), 1);
        assert_eq!(grid.group_at(1, 2).unwrap().volume(), 1);

        assert_eq!(grid.group_at(2, 0).unwrap().volume(), 1);
        assert_eq!(grid.group_at(2, 1).unwrap().volume(), 1);
        assert_eq!(grid.group_at(2, 2).unwrap().volume(), 1);

    }

    #[test]
    fn test_polars_data_1x3x1(){
        /**
         property_1     1   2   3    - X
         property_2
            1           11  12  13 

            Y
         
        **/

        let mut df = df!(
            "property_1" => &[1,   2,   3,  ], 
            "property_2" => &[10,  10,  10, ],
            "property_3" => &[11,  12,  13, ] 
        ).unwrap();

        debug!("wapuku: df: {:?}", df);


        // let mut grid = x_property_1_y_property_2_to_3_x_3_data(df, (Some(1_i64), Some(4_i64)), (Some(10_i64), Some(10_i64)));
        let mut grid = x_property_1_y_property_2_to_3_x_3_data(df, (None, None), (None, None));

        debug!("wapuku: grid: {:?}", grid);

        // let data = grid.data();


        // assert_eq!(grid.group_at(0, 0).is_none(), true);
        // assert_eq!(grid.group_at(0, 1).is_none(), true);
        // assert_eq!(grid.group_at(0, 2).is_none(), true);

        assert_eq!(grid.group_at(1, 0).unwrap().volume(), 1);
        assert_eq!(grid.group_at(1, 1).unwrap().volume(), 1);
        assert_eq!(grid.group_at(1, 2).unwrap().volume(), 1);

        // assert_eq!(grid.group_at(2, 0).is_none(), true);
        // assert_eq!(grid.group_at(2, 1).is_none(), true);
        // assert_eq!(grid.group_at(2, 2).is_none(), true);

    }


    #[test]
    fn test_polars_data_2x3x3(){
        /**
         property_1     1   2   3    - X
         property_2
            1           11  12  13 
            2           21  22  23
            3           31  32  33

            Y
         
        **/

        let mut df = df!(
            "property_1" => &[1,   2,   3,   1,   2,   3,   1,   2,   3,    1,   2,   3,   1,   2,   3,   1,   2,   3, ], 
            "property_2" => &[10,  10,  10,  20,  20,  20,  30,  30,  30,   10,  10,  10,  20,  20,  20,  30,  30,  30,],
            "property_3" => &[11,  12,  13,  21,  22,  23,  31,  32,  33,   110, 120, 130, 210, 220, 230, 310, 320, 330,] 
        ).unwrap();

        debug!("wapuku: df: {:?}", df);

        // let mut grid = x_property_1_y_property_2_to_3_x_3_data(df, (Some(1i64), Some(4i64)), (Some(10i64), Some(31i64)));
        let grid = x_property_1_y_property_2_to_3_x_3_data(df, (None, None), (None, None));

        debug!("wapuku: grid: {:?}", grid);

        // let data = grid.data();

        assert_eq!(grid.group_at(0, 0).unwrap().volume(), 2);
        assert_eq!(grid.group_at(0, 1).unwrap().volume(), 2);
        assert_eq!(grid.group_at(0, 2).unwrap().volume(), 2);
                                                          
        assert_eq!(grid.group_at(1, 0).unwrap().volume(), 2);
        assert_eq!(grid.group_at(1, 1).unwrap().volume(), 2);
        assert_eq!(grid.group_at(1, 2).unwrap().volume(), 2);
                                                          
        assert_eq!(grid.group_at(2, 0).unwrap().volume(), 2);
        assert_eq!(grid.group_at(2, 1).unwrap().volume(), 2);
        assert_eq!(grid.group_at(2, 2).unwrap().volume(), 2);

    }



    #[test]
    fn test_polars_data_1x2x3(){
        /**
         property_1      2   3    - X
         property_2
            1            12  13 
            2            22  23
            3            32  33

            Y
         
        **/

        let mut df = df!(
            "property_1" => &[2,   3,   2,   3,   2,   3,], 
            "property_2" => &[10,   20,   30,   10,   20,   30,],
            "property_3" => &[12,  13,  22,  23,  32,  33,] 
        ).unwrap();


        debug!("wapuku: df: {:?}", df);

        let mut grid = x_property_1_y_property_2_to_3_x_3_data(df, (Some(1i64), Some(4i64)), (Some(10i64), Some(31i64)));
        // let data = grid.data();
        
        debug!("wapuku: grid={:?}", grid);
        
        assert_eq!(grid.group_at(0, 0).is_none(), true);
        assert_eq!(grid.group_at(1, 0).unwrap().volume(), 1);
        assert_eq!(grid.group_at(2, 0).unwrap().volume(), 1);

        assert_eq!(grid.group_at(0, 1).is_none(), true);
        assert_eq!(grid.group_at(1, 1).unwrap().volume(), 1);
        assert_eq!(grid.group_at(2, 1).unwrap().volume(), 1);

        assert_eq!(grid.group_at(0, 2).is_none(), true);
        assert_eq!(grid.group_at(1, 2).unwrap().volume(), 1);
        assert_eq!(grid.group_at(2, 2).unwrap().volume(), 1);

    }

    #[test]
    fn test_polars_data_1x3x2(){
        /**
         property_1     1   2   3    - X
         property_2

            2           21  22  23
            3           31  32  33

            Y
        **/

        let mut df = df!(
            "property_1" => &[1,   2,   3,   1,   2,   3,], 
            "property_2" => &[20,  20,  20,   30,   30,   30,],
            "property_3" => &[21,  22,  23,  31,  32,  33,] 
        ).unwrap();


        debug!("wapuku: df: {:?}", df);

        let mut grid = x_property_1_y_property_2_to_3_x_3_data(df, (Some(1i64), Some(4i64)), (Some(10i64), Some(31i64)));
        // let data = gird.data();


        assert_eq!(grid.group_at(0, 0).is_none(), true);
        assert_eq!(grid.group_at(0, 1).unwrap().volume(), 1);
        assert_eq!(grid.group_at(0, 2).unwrap().volume(), 1);

        assert_eq!(grid.group_at(1, 0).is_none(), true);
        assert_eq!(grid.group_at(1, 1).unwrap().volume(), 1);
        assert_eq!(grid.group_at(1, 2).unwrap().volume(), 1);
 
        assert_eq!(grid.group_at(2, 0).is_none(), true);
        assert_eq!(grid.group_at(2, 1).unwrap().volume(), 1);
        assert_eq!(grid.group_at(2, 2).unwrap().volume(), 1);

    }

    #[test]
    fn test_polars_data_1x4x3(){
        /**
         property_1     1   2   3   4 - X
         property_2
            1           11  12  13  14
            2           21  22  23  24
            3           31  32  33  24

            Y
         
        **/

        let mut df = df!(
            "property_1" => &[1,   2,   3,   4,   1,   2,   3,   4,  1,   2,   3,  4], 
            "property_2" => &[10,  10,  10,  10,  20,  20,  20,  20, 30,  30,  30, 30],
            "property_3" => &[11,  12,  13,  14,  21,  22,  23,  24, 31,  32,  33, 34] 
        ).unwrap();


        debug!("wapuku: df: {:?}", df);

        let mut grid = x_property_1_y_property_2_to_3_x_3_data(df, (Some(0i64), Some(5i64)), (Some(0i64), Some(31i64)));
        // let data = grid.data();
        assert_eq!(grid.group_at(0, 0).unwrap().volume(), 1);
        assert_eq!(grid.group_at(1, 0).unwrap().volume(), 2);
        assert_eq!(grid.group_at(2, 0).unwrap().volume(), 1);

        assert_eq!(grid.group_at(0, 1).unwrap().volume(), 1);
        assert_eq!(grid.group_at(1, 1).unwrap().volume(), 2);
        assert_eq!(grid.group_at(2, 1).unwrap().volume(), 1);

        assert_eq!(grid.group_at(0, 2).unwrap().volume(), 1);
        assert_eq!(grid.group_at(1, 2).unwrap().volume(), 2);
        assert_eq!(grid.group_at(2, 2).unwrap().volume(), 1);

    }


    fn x_property_1_y_property_2_to_3_x_3_data(mut df: DataFrame, min_max_x: (Option<i64>, Option<i64>), min_max_y: (Option<i64>, Option<i64>)) -> GroupsGrid {
        let mut data = PolarsData::new(df, String::from("test"));


        let all_properties: HashSet<&dyn Property> = data.all_properties();

        let (property_1, property_2, property_3) = {
            let mut all_properties_iter = all_properties.into_iter().collect::<Vec<&dyn Property>>();
            all_properties_iter.sort_by(|p1, p2| p1.name().cmp(p2.name()));

            (*all_properties_iter.get(0).expect("property_1"), *all_properties_iter.get(1).expect("property_2"), *all_properties_iter.get(2).expect("property_3"))
        };
        
         data.build_grid(
            PropertyRange::new(property_1, min_max_x.0, min_max_x.1),
            PropertyRange::new(property_2, min_max_y.0, min_max_y.1),
            3, 3, "property_3"
        )
        
    }


    #[test]
    #[cfg(perf)]
    fn test_polars_data(){
        
        let df = df!(
            "property_1" => &(0..10_000_000).into_iter().map(|i|i / 10).collect::<Vec<i64>>(), // 10 X 0, 10 X 1 ...
            "property_2" => &(0..10_000_000).into_iter().map(|i|i - (i/10)*10 ).collect::<Vec<i64>>(), // 
            "property_3" => &(0..10_000_000).into_iter().map(|i|i).collect::<Vec<i32>>(),
        ).unwrap();
        
        debug!("wapuku: df: {:?}", df);
        let t_0 = Instant::now();
        
        let mut data = PolarsData::new(df);
        

        let all_properties:HashSet<&dyn Property> = data.all_properties();

        let (property_1, property_2, property_3) = {
            let mut all_properties_iter = all_properties.into_iter().collect::<Vec<&dyn Property>>();
            all_properties_iter.sort_by(|p1, p2|p1.name().cmp(p2.name()));

            (*all_properties_iter.get(0).expect("property_1"), *all_properties_iter.get(1).expect("property_2"), *all_properties_iter.get(2).expect("property_3"))
        };

        let mut data_grid = data.build_grid(
            PropertyRange::new (property_1,  None, None ),
            PropertyRange::new (property_2,  None, None ),
            3, 3, "property_3"
        );

        let data = data_grid.data();
        
        debug!("wapuku: done in {}", t_0.elapsed().as_millis());

    }

    #[test]
    fn test_build_grid_with_more_properties(){
        let mut df = df!(
            "property_1" => &[10,      20,     30,     40,    50,      60,      70,      80,      90, 100], 
            "property_2" => &[1,       1,      1,      1,     2,       2,       2,       2,       2,    2],
            "property_3" => &["a",     "b",    "c",    "d",   "e",     "f",     "g",     "h",     "i",  "j"],
            "property_4" => &[100,     200,    300,    400,   500,     600,     700,     800,     900,  1000],
            "property_5" => &[1000,    2000,   3000,   4000,  5000,    6000,    7000,    8000,    9000, 10000]
        ).unwrap();
        
        
        debug!("wapuku: {:?}", df);
        
        let mut polars_data = PolarsData::new(df, String::from("test"));

        let all_sets = polars_data.all_sets();
        
        debug!("wapuku: all_sets={:?}", all_sets);
        assert_eq!(all_sets.len(), 1);
        
        let all_properties = polars_data.all_properties();
        assert_eq!(all_properties.len(), 5);

        let (property_1, property_2, property_3) = {
            let mut all_properties_iter = all_properties.into_iter();

            (all_properties_iter.next().expect("property_1"), all_properties_iter.next().expect("property_2"), all_properties_iter.next().expect("property_3"))
        };

        let mut data_grid = polars_data.build_grid(
            PropertyRange::new (property_1,  None, None ),
            PropertyRange::new (property_2,  None, None ),
            3, 3, "property_3"
        );
        //TODO
        debug!("wapuku: data_grid={:?}", data_grid.data());
        
    }

    #[test]
    fn test_group_by_same_order() {

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
        let df = group_by_2(
            &df, "field_2", 2,"field_1", 20,[col("field_3").alias("field_3_value")],
            0i64, 0i64
        ).expect("df");
        
        debug!("3. wapuku: df={:?}", df);
       
        assert_eq!(
            *df.column("field_3_value").expect("field_3_value"),
            Series::new("field_3_value", [
                List(Series::new("", ["a"])),
                List(Series::new("", ["b", "c"])),
                List(Series::new("", ["d"])),
                List(Series::new("", ["h", "ii"])),
                List(Series::new("", ["dd", "e"])),
                List(Series::new("", ["f", "g"])),
            ])
        );
    }



    #[test]
    fn test_simp(){
        let df = df!(
            // "property_1" => &(0..10_000_000).into_iter().map(|i|i / 10).collect::<Vec<i64>>(), // 10 X 0, 10 X 1 ...
            // "property_2" => &(0..10_000_000).into_iter().map(|i|i - (i/10)*10 ).collect::<Vec<i64>>(), // 
            // "property_3" => &(0..10_000_000).into_iter().map(|i|i).collect::<Vec<i32>>(),
            
            // "property_1" => &[1,   2,   3,   1,   2,   3,   1,   2,   3,], 
            // "property_2" => &[10,  10,  10,  20,  20,  20,  30,  30,  30,],
            // "property_3" => &[11,  12,  13,  21,  22,  23,  31,  32,  33,] 
            
            "property_1" => &(0..10000).into_iter().map(|i|i / 100).collect::<Vec<i64>>(), // 10 X 0, 10 X 1 ...
            "property_2" => &(0..10000).into_iter().map(|i|i - (i/100)*100 ).collect::<Vec<i64>>(), // 
            "property_3" => &(0..10000).into_iter().map(|i|i).collect::<Vec<i32>>(),
        ).unwrap();

        
        println!("df={:?}", PolarsData::new(df, String::from("test")).build_grid(
            PropertyRange::new (&DataProperty::new(WapukuDataType::Numeric, "property_1"),  Some(-1), Some(100) ),
            PropertyRange::new (&DataProperty::new(WapukuDataType::Numeric, "property_2"),  Some(-1), Some(100) ),
            3, 3, "property_3"
        ));

       /* // let group_by = df.groupby(["field_1"]).unwrap();
        // let by = group_by.select(["field_2"]);
        // let df = by.groups().unwrap();

        let df = df.clone().lazy().groupby_dynamic(
            [],
            DynamicGroupOptions {
                index_column: "field_1".into(),
                every: Duration::new(2),
                period: Duration::new(2),
                offset: Duration::new(-1),
                truncate: false,
                include_boundaries: true,
                closed_window: ClosedWindow::Left,
                start_by: StartBy::WindowBound,
            }

        )
            .agg([col("field_2").alias("field_2"), col("field_3")])
            .collect().unwrap();

        debug!("wapuku: parquet_scan: df={:?}", df);*/


    }
    

}
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Ord, PartialOrd, PartialEq, Eq)]
pub(crate) struct AppConfiguration<'a> {
    _id: i32,
    default_db_url: & 'a str
}

#[cfg(not(test))]
impl <'a> Default for AppConfiguration<'a>{
    fn default() -> Self {
        AppConfiguration{
            _id: 0,
            default_db_url: "./dev.db"
        }
    }
}

#[cfg(test)]
impl <'a> Default for AppConfiguration<'a>{
    fn default() -> Self {
        AppConfiguration{
            _id: 0,
            default_db_url: "./prisma/dev.db"
        }
    }
}


#[cfg(test)]
mod unit_tests {
    use std::path::PathBuf;
    use crate::app::configuration::AppConfiguration;
    use crate::db;
    use crate::db::app_configuration::SetParam::SetId;
    use crate::db::app_configuration::WhereParam;
    use crate::db::read_filters::IntFilter;


    #[tokio::test]
    #[tracing_test::traced_test]
    async fn persistence_ok() {

        let default_configuration = &AppConfiguration::default();
        let db_url = PathBuf::from(default_configuration.default_db_url);
        let db_url = &db_url.canonicalize().unwrap();
        let db_url = db_url.to_str().expect("db url is: <<unrepresentable path>>");
        let db_url = format!("file:{}", db_url).as_str().to_string();

        let client = db::new_client_with_url(db_url.as_str()).await
            .expect(format!("unable to build app_configuration client using db_url : {db_url}").as_str());

        let configuration = client
            .app_configuration()
            .create(vec![SetId(0)])
            .exec()
            .await
            .expect("unable to exec create query for app_configuration");

        let _serialized_configuration = serde_json::to_string_pretty(&configuration)
            .expect("unable to serialize app_configuration");

        let _count = client.app_configuration()
            .count(vec![WhereParam::Id(IntFilter::Equals(0))])
            .exec().await
            .expect("unable to select app_configuration");

    }
}
pub mod curseforge;
pub mod modrinth;

#[cfg(test)]
mod test {
    use serde::{Deserialize, Serialize};
    use serde_repr::{Deserialize_repr, Serialize_repr};

    #[test]
    fn test_into_query_parameters() {
        #[carbon_macro::into_query_parameters]
        #[derive(Debug, Serialize, Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct SearchParameters {
            game_id: i32,
            page: Option<i32>,
            sort_order: Option<SortOrder>,
            class_id: Option<ClassId>,
        }

        #[derive(Debug, Serialize, Deserialize)]
        enum SortOrder {
            #[serde(rename = "asc")]
            Ascending,
            #[serde(rename = "desc")]
            Descending,
        }

        #[derive(Debug, Serialize_repr, Deserialize_repr)]
        #[repr(u16)]
        enum ClassId {
            Mods = 6,
            Modpacks = 4471,
        }

        let search_params = SearchParameters {
            game_id: 432,
            page: None,
            sort_order: Some(SortOrder::Ascending),
            class_id: Some(ClassId::Mods),
        };

        let query = search_params.into_query_parameters().unwrap();

        assert_eq!(query, "gameId=432&sortOrder=asc&classId=6");
    }
}

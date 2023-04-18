        fn test_name_id() {
            // un-comment me to see what the id should look like, given a name
            //  if id().to_string() != $name {
            //      panic!("id for `{}` should be `{:?}`", $name, bs58::decode($name).into_vec().unwrap());
            //  }
            assert_eq!(id().to_string(), $name)
        }

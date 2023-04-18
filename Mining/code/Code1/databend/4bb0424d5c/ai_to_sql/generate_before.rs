    async fn generate(&mut self) -> Result<Option<DataBlock>> {
        if self.finished {
            return Ok(None);
        }

        // ### Postgres SQL tables, with their properties:
        // #
        // # Employee(id, name, department_id)
        // # Department(id, name, address)
        // # Salary_Payments(id, employee_id, amount, date)
        // #
        // ### A query to list the names of the departments which employed more than 10 employees in the last 3 months
        // SELECT
        let database = self.ctx.get_current_database();
        let tenant = self.ctx.get_tenant();
        let catalog = self.ctx.get_catalog(CATALOG_DEFAULT)?;

        let mut template = vec![];
        template.push("### Postgres SQL tables, with their properties:".to_string());
        template.push("#".to_string());

        for table in catalog.list_tables(tenant.as_str(), &database).await? {
            let fields = if table.engine() == VIEW_ENGINE {
                continue;
            } else {
                table.schema().fields().clone()
            };

            let columns_name = fields
                .iter()
                .map(|f| f.name().to_string())
                .collect::<Vec<_>>();
            template.push(format!("{}({})", table.name(), columns_name.join(",")));
        }
        template.push("#".to_string());
        template.push(format!("### {}", self.prompt.clone()));
        template.push("#".to_string());
        template.push("SELECT".to_string());

        let prompt = template.join("");
        info!("openai request prompt: {}", prompt);

        // Response.
        let api_base = GlobalConfig::instance().query.openai_api_base_url.clone();
        let api_key = GlobalConfig::instance().query.openai_api_key.clone();
        let api_embedding_model = GlobalConfig::instance()
            .query
            .openai_api_embedding_model
            .clone();
        let api_completion_model = GlobalConfig::instance()
            .query
            .openai_api_completion_model
            .clone();
        let openai = OpenAI::create(api_base, api_key, api_embedding_model, api_completion_model);
        let (sql, _) = openai.completion_sql_request(prompt)?;

        let sql = format!("SELECT{}", sql);
        info!("openai response sql: {}", sql);
        let database = self.ctx.get_current_database();
        let database: Vec<Vec<u8>> = vec![database.into_bytes()];
        let sql: Vec<Vec<u8>> = vec![sql.into_bytes()];

        // Mark done.
        self.finished = true;

        Ok(Some(DataBlock::new_from_columns(vec![
            StringType::from_data(database),
            StringType::from_data(sql),
        ])))
    }

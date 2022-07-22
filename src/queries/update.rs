use std::marker::PhantomData;

use prisma_models::PrismaValue;
use query_core::{Operation, Selection, SelectionBuilder};
use serde::de::DeserializeOwned;

use crate::{
    merged_object, option_on_not_found,
    select::{SelectOption, SelectType},
};

use super::{QueryContext, QueryInfo, SerializedWhere};

pub struct Update<'a, Where, With, Set, Data>
where
    Where: Into<SerializedWhere>,
    With: Into<Selection>,
    Set: Into<(String, PrismaValue)>,
    Data: DeserializeOwned,
{
    ctx: QueryContext<'a>,
    info: QueryInfo,
    pub where_param: Where,
    pub set_params: Vec<Set>,
    pub with_params: Vec<With>,
    _data: PhantomData<Data>,
}
impl<'a, Where, With, Set, Data> Update<'a, Where, With, Set, Data>
where
    Where: Into<SerializedWhere>,
    Set: Into<(String, PrismaValue)>,
    With: Into<Selection>,
    Data: DeserializeOwned,
{
    pub fn new(
        ctx: QueryContext<'a>,
        info: QueryInfo,
        where_param: Where,
        set_params: Vec<Set>,
        with_params: Vec<With>,
    ) -> Self {
        Self {
            ctx,
            info,
            where_param,
            set_params,
            with_params,
            _data: PhantomData,
        }
    }

    pub fn with(mut self, param: impl Into<With>) -> Self {
        self.with_params.push(param.into());
        self
    }

    fn to_selection(model: &str, where_param: Where, set_params: Vec<Set>) -> SelectionBuilder {
        let mut selection = Selection::builder(format!("updateOne{}", model));

        selection.alias("result");

        selection.push_argument(
            "where",
            PrismaValue::Object(vec![where_param.into().transform_equals()]),
        );

        selection.push_argument(
            "data",
            merged_object(set_params.into_iter().map(Into::into).collect()),
        );

        selection
    }

    pub fn select<S: SelectType<Data>>(self, select: S) -> SelectOption<'a, S::Data> {
        let mut selection = Self::to_selection(self.info.model, self.where_param, self.set_params);

        selection.nested_selections(select.to_selections());

        let op = Operation::Write(selection.build());

        SelectOption::new(self.ctx, op)
    }

    pub async fn exec(self) -> super::Result<Option<Data>> {
        let QueryInfo {
            model,
            mut scalar_selections,
        } = self.info;

        let mut selection = Self::to_selection(model, self.where_param, self.set_params);

        if self.with_params.len() > 0 {
            scalar_selections.append(&mut self.with_params.into_iter().map(Into::into).collect());
        }
        selection.nested_selections(scalar_selections);

        let op = Operation::Write(selection.build());

        option_on_not_found(self.ctx.execute(op).await)
    }
}

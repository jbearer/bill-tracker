//! Compilation of select from high-level GraphQL types into low-level SQL types.

use super::{
    super::db::{Column, Connection, JoinClause, Row, Select, SelectColumn, SelectExt},
    field_column, scalar_to_value, table_name, value_to_scalar, Error,
};
use crate::graphql::type_system::{self as gql, ResourcePredicate, ScalarPredicate, Type};
use std::any::TypeId;
use std::collections::HashMap;
use std::marker::PhantomData;
use take_mut::take;

enum Relation<'a> {
    ManyToOne {
        /// The field on the target resource that identifies the owning object.
        inverse: Column<'a>,
        /// The owner whose targets we want.
        owner: gql::Id,
    },
    ManyToMany,
}

/// Search for items of resource `T` matching `filter`.
pub async fn execute<C: Connection, T: gql::Resource>(
    conn: &C,
    filter: Option<T::Predicate>,
) -> Result<Vec<T>, Error> {
    execute_and_filter(conn, filter, None).await
}

/// Load the targets of a [`Relation`](gql::Relation).
pub async fn load_relation<C: Connection, R: gql::Relation>(
    conn: &C,
    owner: &R::Owner,
    filter: Option<<R::Target as gql::Type>::Predicate>,
) -> Result<Vec<R::Target>, Error> {
    // Match on the type of the relation. Many-to-one relations are handled via a simple filter.
    // Many-to-many relations are more complicated because we have to go through a join table.
    struct Visitor<'a, T>(&'a T);

    impl<'a, T: gql::Resource> gql::RelationVisitor<T> for Visitor<'a, T> {
        type Output = Relation<'static>;

        fn visit_many_to_one<R: gql::ManyToOneRelation<Owner = T>>(&mut self) -> Self::Output {
            Relation::ManyToOne {
                inverse: field_column::<R::Inverse>(),
                owner: *self.0.get::<T::Id>(),
            }
        }

        fn visit_many_to_many<R: gql::ManyToManyRelation<Owner = T>>(&mut self) -> Self::Output {
            Relation::ManyToMany
        }
    }

    let relation = R::visit(&mut Visitor(owner));
    execute_and_filter(conn, filter, Some(relation)).await
}

/// Search for items of resource `T` matching `filter`.
///
/// Optionally, restrict output to items in a relation.
async fn execute_and_filter<C: Connection, T: gql::Resource>(
    conn: &C,
    filter: Option<T::Predicate>,
    relation: Option<Relation<'_>>,
) -> Result<Vec<T>, Error> {
    // Traverse the resource and map its fields to tables and columns.
    let table = table_name::<T>();
    let mut columns = ColumnMap::new::<T>();
    let select = columns.columns.clone();

    // Select the columns we need to reconstruct this resource from the query results and apply the
    // `filter`.
    let mut query = conn.select(&select, &table);
    if let Some(predicate) = filter {
        query = compile_predicate::<_, T>(&mut columns, query, predicate);
    }

    // Filter down to just the relation of interest.
    match relation {
        Some(Relation::ManyToOne { inverse, owner }) => {
            // We want all the objects in the target resource where the inverse of the relation (the
            // field that indicates the owner of the target) matches the ID of the owning object.
            query = query.filter(inverse, "=", owner.into());
        }
        Some(Relation::ManyToMany) => {
            unimplemented!("many-to-many relations");
        }
        None => {}
    }

    query = query.clauses(std::mem::take(&mut columns.joins).into_values());
    let rows = query.many().await.map_err(Error::sql)?;
    rows.iter().map(|row| columns.parse_row(row)).collect()
}

/// A map from field types to the select column for that field.
#[derive(Clone, Debug, Default)]
struct ColumnMap {
    index: HashMap<TypeId, usize>,
    columns: Vec<SelectColumn<'static>>,
    joins: HashMap<TypeId, JoinClause<'static>>,
}

impl ColumnMap {
    /// A column map for `T` and all of its nested resources.
    fn new<T: gql::Resource>() -> Self {
        let mut columns = Self::default();
        columns.add_resource::<T>();
        columns
    }

    /// Add columns for `T` and all of its nested resources.
    fn add_resource<T: gql::Resource>(&mut self) {
        // For each field of `T`, get a list of field columns including the column for that field as
        // well as columns for every field on the type of the column, if that type is a resource.
        struct Visitor<'a>(&'a mut ColumnMap);

        impl<'a, T: gql::Resource> gql::FieldVisitor<T> for Visitor<'a> {
            type Output = ();

            fn visit<F: gql::Field<Resource = T>>(&mut self) -> Self::Output {
                // Check if the type of `F` is also a resource, and if so get its columns
                // recursively.
                struct Visitor<'a, F> {
                    columns: &'a mut ColumnMap,
                    _phantom: PhantomData<fn(&F)>,
                }

                impl<'a, F: gql::Field> gql::Visitor<F::Type> for Visitor<'a, F> {
                    type Output = ();

                    fn resource(self) -> Self::Output
                    where
                        F::Type: gql::Resource,
                    {
                        // Add a join clause to bring this table into the result set.
                        self.columns.join::<F>();

                        // Select the fields of the joined table.
                        self.columns.add_resource::<F::Type>();
                    }

                    fn scalar(self) -> Self::Output
                    where
                        F::Type: gql::Scalar,
                    {
                        // Nothing to do if this column is a scalar; all there is is the column
                        // itself, which we add below.
                    }
                }

                F::Type::describe(Visitor::<F> {
                    columns: self.0,
                    _phantom: Default::default(),
                });

                // Add the column for this field.
                self.0.push::<F>();
            }
        }

        T::describe_fields(&mut Visitor(self));
    }

    /// Add a column for the field `F`.
    fn push<F: gql::Field>(&mut self) {
        self.index.insert(TypeId::of::<F>(), self.columns.len());
        self.columns.push(SelectColumn::Column(field_column::<F>()));
    }

    /// Add a relation to the query without including its fields in the results.
    fn join<F: gql::Field>(&mut self)
    where
        F::Type: gql::Resource,
    {
        // We join on the column referencing this resource in the original table (`F`) being equal
        // to the primary key (`Id`) of this resource's table.
        self.joins.insert(
            TypeId::of::<F>(),
            JoinClause {
                table: table_name::<F::Type>().into(),
                lhs: field_column::<F>(),
                op: "=".into(),
                rhs: field_column::<<F::Type as gql::Resource>::Id>(),
            },
        );
    }

    /// Convert a row of query results into a resource object.
    fn parse_row<R: Row, T: gql::Resource>(&self, row: &R) -> Result<T, Error> {
        T::build_resource(ResourceBuilder::new(self, row))
    }

    /// The index of the column representing field `F`.
    fn index<F: gql::Field>(&self) -> usize {
        self.index[&TypeId::of::<F>()]
    }
}

impl AsRef<[SelectColumn<'static>]> for ColumnMap {
    fn as_ref(&self) -> &[SelectColumn<'static>] {
        &self.columns
    }
}

/// Compiler to turn a scalar predicate into a condition which is part of a `WHERE` clause.
struct ScalarWhereCondition<'a, Q> {
    column: Column<'a>,
    query: Q,
}

impl<'a, Q: Select<'a>, T: gql::Scalar> gql::ScalarPredicateCompiler<T>
    for ScalarWhereCondition<'a, Q>
{
    type Result = Q;

    fn cmp(self, op: T::Cmp, value: gql::Value<T>) -> Self::Result {
        match value {
            gql::Value::Lit(x) => {
                self.query
                    .filter(self.column, op.to_string(), scalar_to_value(x))
            }
            gql::Value::Var(_) => unimplemented!("pattern variables"),
        }
    }
}

/// Compiler to turn a predicate into a condition which is part of a `WHERE` clause.
struct WhereCondition<'a, Q, F: gql::Field> {
    columns: &'a mut ColumnMap,
    query: Q,
    predicate: <F::Type as gql::Type>::Predicate,
}

impl<'a, 'b, Q: Select<'a>, F: gql::Field> gql::Visitor<F::Type> for WhereCondition<'b, Q, F> {
    type Output = Q;

    fn resource(self) -> Q
    where
        F::Type: gql::Resource,
    {
        // Join `T` into the query.
        self.columns.join::<F>();
        compile_predicate::<Q, F::Type>(self.columns, self.query, self.predicate)
    }

    fn scalar(self) -> Self::Output
    where
        F::Type: gql::Scalar,
    {
        self.predicate.compile(ScalarWhereCondition {
            column: field_column::<F>(),
            query: self.query,
        })
    }
}

/// Compile a predicate on a resource into a `WHERE` clause on a query of that table.
fn compile_predicate<'a, 'b, Q: Select<'a>, T: gql::Resource>(
    columns: &'b mut ColumnMap,
    query: Q,
    pred: T::ResourcePredicate,
) -> Q {
    struct Visitor<'a, Q, T: gql::Resource> {
        columns: &'a mut ColumnMap,
        query: Q,
        pred: T::ResourcePredicate,
    }

    impl<'a, 'b, Q: Select<'a>, T: gql::Resource> gql::ResourceVisitor<T> for Visitor<'b, Q, T> {
        type Output = Q;

        fn visit_field_in_place<F: gql::Field<Resource = T>>(&mut self) {
            if let Some(sub_pred) = self.pred.take::<F>() {
                take(&mut self.query, |query| {
                    F::Type::describe(WhereCondition::<Q, F> {
                        columns: self.columns,
                        query,
                        predicate: sub_pred,
                    })
                });
            }
        }

        fn visit_many_to_one_in_place<R: gql::ManyToOneRelation<Owner = T>>(&mut self) {
            unimplemented!("relations predicates")
        }

        fn visit_many_to_many_in_place<R: gql::ManyToManyRelation<Owner = T>>(&mut self) {
            unimplemented!("relations predicates")
        }

        fn end(self) -> Q {
            self.query
        }
    }

    T::describe_resource(Visitor {
        columns,
        query,
        pred,
    })
}

/// Builder to help a resource object reconstruct itself from query results.
struct ResourceBuilder<'a, R> {
    row: &'a R,
    columns: &'a ColumnMap,
}

impl<'a, R> ResourceBuilder<'a, R> {
    fn new(columns: &'a ColumnMap, row: &'a R) -> Self {
        Self { row, columns }
    }
}

impl<'a, R: Row, T: gql::Resource> gql::ResourceBuilder<T> for ResourceBuilder<'a, R> {
    type Error = Error;

    fn field<F: gql::Field<Resource = T>>(&self) -> Result<F::Type, Error>
    where
        F: gql::Field<Resource = T>,
    {
        // Builder to reconstruct the type of `F`.
        struct Builder<'a, R> {
            column: usize,
            columns: &'a ColumnMap,
            row: &'a R,
        }

        impl<'a, R: Row, T: 'a + gql::Type> gql::Builder<T> for Builder<'a, R> {
            type Error = Error;
            type Resource = ResourceBuilder<'a, R> where T: gql::Resource;

            fn resource(self) -> Self::Resource
            where
                T: gql::Resource,
            {
                ResourceBuilder::new(self.columns, self.row)
            }

            fn scalar(self) -> Result<T, Error>
            where
                T: gql::Scalar,
            {
                value_to_scalar(self.row.column(self.column).map_err(Error::sql)?)
            }
        }

        <F::Type as gql::Type>::build(Builder {
            column: self.columns.index::<F>(),
            columns: self.columns,
            row: self.row,
        })
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::{
        array, init_logging,
        sql::db::{mock, SchemaColumn, Type, Value},
    };
    use generic_array::typenum::{U2, U3};
    use gql::{Id, Resource};

    /// A simple test resource with scalar fields.
    #[derive(Clone, Debug, PartialEq, Eq, Resource)]
    struct TestResource {
        id: Id,
        field1: i32,
        field2: String,
    }

    #[async_std::test]
    async fn test_resource_predicate() {
        init_logging();

        let resources = [
            TestResource {
                id: 1,
                field1: 0,
                field2: "foo".into(),
            },
            TestResource {
                id: 2,
                field1: 1,
                field2: "bar".into(),
            },
            TestResource {
                id: 3,
                field1: 1,
                field2: "baz".into(),
            },
        ];

        let db = mock::Connection::create();
        db.create_table_with_rows::<U3>(
            "test_resources",
            array![SchemaColumn;
                SchemaColumn::new("id", Type::Serial),
                SchemaColumn::new("field1", Type::Int4),
                SchemaColumn::new("field2", Type::Text),
            ],
            [
                array![Value;
                    Value::from(resources[0].field1),
                    Value::from(resources[0].field2.clone()),
                ],
                array![Value;
                    Value::from(resources[1].field1),
                    Value::from(resources[1].field2.clone()),
                ],
                array![Value;
                    Value::from(resources[2].field1),
                    Value::from(resources[2].field2.clone()),
                ],
            ],
        )
        .await
        .unwrap();

        // Test a single sub-predicate.
        let predicate = TestResource::has()
            .field1(<i32 as gql::Type>::Predicate::cmp(
                gql::IntCmpOp::EQ,
                gql::Value::Lit(1),
            ))
            .into();
        assert_eq!(
            execute::<_, TestResource>(&db, Some(predicate))
                .await
                .unwrap(),
            &resources[1..]
        );

        // Test multiple sub-predicates.
        let predicate = TestResource::has()
            .field1(<i32 as gql::Type>::Predicate::cmp(
                gql::IntCmpOp::EQ,
                gql::Value::Lit(1),
            ))
            .field2(<String as gql::Type>::Predicate::cmp(
                gql::StringCmpOp::NE,
                gql::Value::Lit("baz".into()),
            ))
            .into();
        assert_eq!(
            execute::<_, TestResource>(&db, Some(predicate))
                .await
                .unwrap(),
            &resources[1..2],
        );

        // Test no sub-predicates.
        let predicate = TestResource::has().into();
        assert_eq!(
            execute::<_, TestResource>(&db, Some(predicate))
                .await
                .unwrap(),
            &resources,
        );
    }

    #[derive(Clone, Debug, PartialEq, Eq, Resource)]
    struct Left {
        id: Id,
        field: i32,
    }

    #[derive(Clone, Debug, PartialEq, Eq, Resource)]
    struct Right {
        id: Id,
        field: i32,
    }

    #[derive(Clone, Debug, PartialEq, Eq, Resource)]
    struct Node {
        id: Id,
        left: Left,
        right: Right,
    }

    #[async_std::test]
    async fn test_join_filter() {
        init_logging();

        let db = mock::Connection::create();
        db.create_table_with_rows::<U2>(
            "lefts",
            array![SchemaColumn;
                SchemaColumn::new("id", Type::Serial),
                SchemaColumn::new("field", Type::Int4),
            ],
            [array![Value; Value::from(0)], array![Value; Value::from(1)]],
        )
        .await
        .unwrap();
        db.create_table_with_rows::<U2>(
            "rights",
            array![SchemaColumn;
                SchemaColumn::new("id", Type::Serial),
                SchemaColumn::new("field", Type::Int4),
            ],
            [array![Value; Value::from(0)], array![Value; Value::from(1)]],
        )
        .await
        .unwrap();
        db.create_table_with_rows::<U3>(
            "nodes",
            array![SchemaColumn;
                SchemaColumn::new("id", Type::Serial),
                SchemaColumn::new("left", Type::Int4),
                SchemaColumn::new("right", Type::Int4),
            ],
            [
                array![Value; Value::from(1), Value::from(2)],
                array![Value; Value::from(2), Value::from(2)],
            ],
        )
        .await
        .unwrap();

        // Select all.
        assert_eq!(
            execute::<_, Node>(&db, None).await.unwrap(),
            [
                Node {
                    id: 1,
                    left: Left { id: 1, field: 0 },
                    right: Right { id: 2, field: 1 },
                },
                Node {
                    id: 2,
                    left: Left { id: 2, field: 1 },
                    right: Right { id: 2, field: 1 },
                },
            ]
        );

        // Select with a WHERE clause.
        assert_eq!(
            execute::<_, Node>(
                &db,
                Some(
                    Node::has()
                        .left(
                            Left::has()
                                .field(<i32 as gql::Type>::Predicate::cmp(
                                    gql::IntCmpOp::EQ,
                                    gql::Value::Lit(0)
                                ))
                                .into()
                        )
                        .into()
                )
            )
            .await
            .unwrap(),
            [Node {
                id: 1,
                left: Left { id: 1, field: 0 },
                right: Right { id: 2, field: 1 }
            }]
        );
    }
}
use self::private::LoadIter;
use super::RunQueryDsl;
use crate::backend::Backend;
use crate::connection::{Connection, ConnectionGatWorkaround};
use crate::deserialize::FromSqlRow;
use crate::expression::QueryMetadata;
use crate::query_builder::{AsQuery, QueryFragment, QueryId};
use crate::result::QueryResult;

#[cfg(feature = "i-implement-a-third-party-backend-and-opt-into-breaking-changes")]
pub use self::private::{CompatibleType, LoadQueryGatWorkaround};

#[cfg(not(feature = "i-implement-a-third-party-backend-and-opt-into-breaking-changes"))]
pub(crate) use self::private::{CompatibleType, LoadQueryGatWorkaround};

/// The `load` method
///
/// This trait should not be relied on directly by most apps. Its behavior is
/// provided by [`RunQueryDsl`]. However, you may need a where clause on this trait
/// to call `load` from generic code.
///
/// [`RunQueryDsl`]: crate::RunQueryDsl
pub trait LoadQuery<'query, Conn, U>: RunQueryDsl<Conn>
where
    for<'a> Self: LoadQueryGatWorkaround<'a, 'query, Conn, U>,
{
    /// Load this query
    fn internal_load<'conn>(
        self,
        conn: &'conn mut Conn,
    ) -> QueryResult<<Self as LoadQueryGatWorkaround<'conn, 'query, Conn, U>>::Ret>;
}

/// The return type of [`LoadQuery<C, U>::internal_load()`]
///
/// Users should thread this type as `impl Iterator<Item = QueryResult<U>>`
pub type LoadRet<'conn, 'query, Q, C, U> = <Q as LoadQueryGatWorkaround<'conn, 'query, C, U>>::Ret;

impl<'conn, 'query, Conn, T, U, DB> LoadQueryGatWorkaround<'conn, 'query, Conn, U> for T
where
    Conn: Connection<Backend = DB>,
    T: AsQuery + RunQueryDsl<Conn>,
    T::Query: QueryFragment<DB> + QueryId,
    T::SqlType: CompatibleType<U, DB>,
    DB: Backend + QueryMetadata<T::SqlType> + 'static,
    U: FromSqlRow<<T::SqlType as CompatibleType<U, DB>>::SqlType, DB> + 'static,
    <T::SqlType as CompatibleType<U, DB>>::SqlType: 'static,
{
    type Ret = LoadIter<
        'conn,
        U,
        <Conn as ConnectionGatWorkaround<'conn, 'query, DB>>::Cursor,
        <T::SqlType as CompatibleType<U, DB>>::SqlType,
        DB,
    >;
}

impl<'query, Conn, T, U, DB> LoadQuery<'query, Conn, U> for T
where
    Conn: Connection<Backend = DB>,
    T: AsQuery + RunQueryDsl<Conn>,
    T::Query: QueryFragment<DB> + QueryId + 'query,
    T::SqlType: CompatibleType<U, DB>,
    DB: Backend + QueryMetadata<T::SqlType> + 'static,
    U: FromSqlRow<<T::SqlType as CompatibleType<U, DB>>::SqlType, DB> + 'static,
    <T::SqlType as CompatibleType<U, DB>>::SqlType: 'static,
{
    fn internal_load<'conn>(
        self,
        conn: &'conn mut Conn,
    ) -> QueryResult<<Self as LoadQueryGatWorkaround<'conn, 'query, Conn, U>>::Ret> {
        Ok(LoadIter {
            cursor: conn.load(self.as_query())?,
            _marker: Default::default(),
        })
    }
}

/// The `execute` method
///
/// This trait should not be relied on directly by most apps. Its behavior is
/// provided by [`RunQueryDsl`]. However, you may need a where clause on this trait
/// to call `execute` from generic code.
///
/// [`RunQueryDsl`]: crate::RunQueryDsl
pub trait ExecuteDsl<Conn: Connection<Backend = DB>, DB: Backend = <Conn as Connection>::Backend>:
    Sized
{
    /// Execute this command
    fn execute(query: Self, conn: &mut Conn) -> QueryResult<usize>;
}

use crate::result::Error;

impl<Conn, DB, T> ExecuteDsl<Conn, DB> for T
where
    Conn: Connection<Backend = DB>,
    DB: Backend,
    T: QueryFragment<DB> + QueryId,
{
    fn execute(query: T, conn: &mut Conn) -> Result<usize, Error> {
        conn.execute_returning_count(&query)
    }
}

// These types and traits are not part of the public API.
//
// * LoadQueryGatWorkaround to allow us replacing it with real GAT later on
// * CompatibleType as we consider this as "sealed" trait. It shouldn't
// be implemented by a third party
// * LoadIter as it's an implementation detail
mod private {
    use crate::backend::Backend;
    use crate::deserialize::FromSqlRow;
    use crate::expression::select_by::SelectBy;
    use crate::expression::{Expression, TypedExpressionType};
    use crate::sql_types::{SqlType, Untyped};
    use crate::{QueryResult, Selectable};

    #[cfg_attr(
        feature = "i-implement-a-third-party-backend-and-opt-into-breaking-changes",
        cfg(feature = "i-implement-a-third-party-backend-and-opt-into-breaking-changes")
    )]
    pub trait LoadQueryGatWorkaround<'conn, 'query, Conn, U> {
        type Ret: Iterator<Item = QueryResult<U>>;
    }

    #[allow(missing_debug_implementations)]
    pub struct LoadIter<'a, U, C, ST, DB> {
        pub(super) cursor: C,
        pub(super) _marker: std::marker::PhantomData<&'a (ST, U, DB)>,
    }

    impl<'a, C, U, ST, DB, R> LoadIter<'a, U, C, ST, DB>
    where
        DB: Backend,
        C: Iterator<Item = QueryResult<R>>,
        R: crate::row::Row<'a, DB>,
        U: FromSqlRow<ST, DB>,
    {
        pub(super) fn map_row(row: Option<QueryResult<R>>) -> Option<QueryResult<U>> {
            match row? {
                Ok(row) => Some(
                    U::build_from_row(&row).map_err(crate::result::Error::DeserializationError),
                ),
                Err(e) => Some(Err(e)),
            }
        }
    }

    impl<'a, C, U, ST, DB, R> Iterator for LoadIter<'a, U, C, ST, DB>
    where
        DB: Backend,
        C: Iterator<Item = QueryResult<R>>,
        R: crate::row::Row<'a, DB>,
        U: FromSqlRow<ST, DB>,
    {
        type Item = QueryResult<U>;

        fn next(&mut self) -> Option<Self::Item> {
            Self::map_row(self.cursor.next())
        }

        fn size_hint(&self) -> (usize, Option<usize>) {
            self.cursor.size_hint()
        }

        fn count(self) -> usize
        where
            Self: Sized,
        {
            self.cursor.count()
        }

        fn last(self) -> Option<Self::Item>
        where
            Self: Sized,
        {
            Self::map_row(self.cursor.last())
        }

        fn nth(&mut self, n: usize) -> Option<Self::Item> {
            Self::map_row(self.cursor.nth(n))
        }
    }

    impl<'a, C, U, ST, DB, R> ExactSizeIterator for LoadIter<'a, U, C, ST, DB>
    where
        DB: Backend,
        C: ExactSizeIterator + Iterator<Item = QueryResult<R>>,
        R: crate::row::Row<'a, DB>,
        U: FromSqlRow<ST, DB>,
    {
        fn len(&self) -> usize {
            self.cursor.len()
        }
    }

    #[cfg_attr(
        doc_cfg,
        doc(cfg(feature = "i-implement-a-third-party-backend-and-opt-into-breaking-changes"))
    )]
    pub trait CompatibleType<U, DB> {
        type SqlType;
    }

    impl<ST, U, DB> CompatibleType<U, DB> for ST
    where
        DB: Backend,
        ST: SqlType + crate::sql_types::SingleValue,
        U: FromSqlRow<ST, DB>,
    {
        type SqlType = ST;
    }

    impl<U, DB> CompatibleType<U, DB> for Untyped
    where
        U: FromSqlRow<Untyped, DB>,
        DB: Backend,
    {
        type SqlType = Untyped;
    }

    impl<U, DB, E, ST> CompatibleType<U, DB> for SelectBy<U, DB>
    where
        DB: Backend,
        ST: SqlType + TypedExpressionType,
        U: Selectable<DB, SelectExpression = E>,
        E: Expression<SqlType = ST>,
        U: FromSqlRow<ST, DB>,
    {
        type SqlType = ST;
    }
}

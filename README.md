# diesel-selectable-macro

When inserting, Diesel allows you to derive the `Insertable` trait, which
inserts keys by name:

```rs
use diesel::prelude::*;

#[derive(Insertable)]
#[table_name = "users"]
struct User {
  email: String,
  password_hash: String,
  // There's another field, `phone`, but we are not writing it.
}

// later on...

fn write(user: User) -> QueryResult<usize> {
  diesel::insert_into(users::table).values(user).execute(conn)
}
```

This crate offers a similar derive trait for _reading_ data. Diesel's
`Queryable` trait reads by position rather than field name, but sometimes field
name is more convenient:

```rs
use diesel::prelude::*;
use diesel_selectable_macro::Selectable;

#[derive(Selectable)]
#[table_name = "users"]
struct User {
  email: String,
  password_hash: String,
  // There's another field, `phone`, but we do not need to read it.
}

// later on...

fn read(email: String) -> QueryResult<User> {
  User::select().filter(crate::schema::users::email.eq(&email)).get_result(conn)
}
```

The automatically derived `select` method provides the explicit fields to
Diesel, corresponding to the struct fields.

# Data model for bill-related information.

The data model is presented in two equivalent instantiations, one for GraphQL and one for
(Postgre)SQL. These are two different ways of viewing the same data, and they must be kept in sync
with care.

The `graphql` model describes clients' view of the data. It provides an ontology that clients can
use to conceptualize the various entities and their relationships as well as an expressive language
for querying the data.

The `sql` model describes how the data is actually stored in the backend. It gives the server the
ability to leverage an RDBMS to efficiently solve GraphQL queries from clients.

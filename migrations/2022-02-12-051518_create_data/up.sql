-- Your SQL goes here
CREATE TABLE data (
  datetime timestamp with time zone PRIMARY KEY,
  temperature real,
  brightness real,
  co2 integer,
  tvoc integer
)

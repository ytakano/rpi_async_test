table! {
    data (datetime) {
        datetime -> Timestamptz,
        temperature -> Nullable<Float4>,
        brightness -> Nullable<Float4>,
        co2 -> Nullable<Int4>,
        tvoc -> Nullable<Int4>,
    }
}

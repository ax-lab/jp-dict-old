# jp-dict

Rust library with Japanese dictionary data. Contains words, kanji and frequency
information compiled from several dictionaries.

For ease of use the dictionary data is embedded in the library.

## Dictionary data

This project imports dictionary data from Yomichan compatible dictionaries
(see https://foosoft.net/projects/yomichan/). Those are zip files in a format
internal to Yomichan that are compiled from the source dictionaries.

Running `make import` will import the data from the `data` directory and
generate a `dictionary.in` file which is required to build the library.

The source dictionary data is not included in the project and must be downloaded
to the `data` directory (see [README](data/README.md)).

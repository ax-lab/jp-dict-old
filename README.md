# x-jp-data

Japanese dictionary library with words, kanji and frequency information.

For ease of use, the dictionary data is built into the compiled library.

## Dictionary data

This project imports dictionary data from Yomichan compatible dictionaries
(see https://foosoft.net/projects/yomichan/). Those are zip files in a format
internal to Yomichan that are compiled from the source dictionaries.

Running `make import` will import the data from the `data` directory. The
dictionary data is not included in the project by default and must be downloaded
to the `data` directory (see [README](data/README.md)).

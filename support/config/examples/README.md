# precached - Predefined Configuration Files

This directory contains some predefined configuration profiles for precached.
You can use this as a starting point for your own customization efforts.

## Advice on when to choose which configuration profile

### precached-aggressive.conf

Choose this configuration when you have huge amounts of RAM installed.
It does aggressive caching and forces nearly all of your system files
to stay resident in RAM.

### precached-lowmem-dynonly.conf

Choose this configuration when you don't have much available RAM.
It will only lock the most fequently mapped (used) files to RAM.

### precached-devel.conf

Choose this configuration for precached development. It enables all available
plugins and achieves a reasonable amount of code coverage.

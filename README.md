# ImMosaic

Create a mosaic from a set of images.

# Usage:

```bash
./immosaic --sources <path-to-source-image-dir> --output mosaic.png --input-image source.jpg --tiles-per-side 50
```

The cache for the images in the folder is build on the first run. It will be used if available. `--rebuild-cache` will regenerate the file. The initial creation may take some time depending on the number of images in the folder and their size. The resulting image has the same size as the original input image.

Currently only bmp, jpg, png, and webp are supported at the moment.

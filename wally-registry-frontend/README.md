# Wally Registry Frontend

The Wally registry website serves as a place for Wally users to view information about the Wally package manager.

## Usage

To run in development mode:

```bash
npm install

## Pick one:
# for front end and back end:
npm run dev
# for just front end
npm run dev:front
```

To build to the `dist/` directory:

```bash
npm run build
```

To crush all uncommitted PNGs (do this before committing them):

```bash
npm run crush
```

All files should be formatted with Prettier before committing. You can either (pick one):

- Install the Prettier editor extension, and turn on the "format file on save" setting in your editor
- Run the `npm run format` command before committing any files, which will format all changed files in-place.

## Recommended Editor Extensions

These extensions make working on this project easier and help prevent mistakes. Links for VS Code extensions are provided.

- [Styled Components](https://marketplace.visualstudio.com/items?itemName=jpoissonnier.vscode-styled-components) is important for syntax highlighting and IntelliSense for the Styled Components CSS.
- [MDX](https://marketplace.visualstudio.com/items?itemName=silvenon.mdx) is important for proper syntax highlighting in MDX files.
- [Firebase](https://marketplace.visualstudio.com/items?itemName=toba.vsfire) syntax highlighting for Firestore configuration files.
- [Prettier](https://marketplace.visualstudio.com/items?itemName=esbenp.prettier-vscode)
- [ESLint](https://marketplace.visualstudio.com/items?itemName=dbaeumer.vscode-eslint)

## Structure

### ./generator.js

Custom static site generator using Parcel. This file is what builds the website for a production release. It handles server-side rendering and crawling links to discover all pages to include. If a page isn't accessible via a link, but should still be included in the build, it needs to be added to this file.

### ./src

The main source code for the website.

#### ./src/assets

Any images that are imported by the website should go here. Make sure you crush them before committing (see below)

#### ./src/firebase.ts

This file should only be imported on the client (not server-side rendering). If it's imported at build time, it will error. This module should not be imported during SSR.

### ./icons

This folder contains SVG icons that are built into an icon font using fantasticon. They are usable with the `<Icon />` component.

### ./functions

For Firebase server-side functions. [See more](https://firebase.google.com/docs/functions)

### ./scripts

#### ./scripts/crush.js

Run this after adding new PNG files to the repo. It will crush the images to their lowest possible file size.

### ./static

This folder will be copied into the build and anything in it will be available on the public website.

## Images

PNG files should be crushed before you commit them with the `npm run crush` command. As part of the build process, PNG files are cloned and converted into AVIF and WEBP. To ensure that the browser can load these more efficient image formats, use the `Img` component instead of `img`. Also avoid using background images with CSS as these cannot specify multiple image formats. Instead, use the `BgImg` component. Note the more efficient formats are only used in a production build, not development mode.

## MDX

This project makes use of MDX files, which is Markdown mixed with JSX. MDX can be used for pages, and is used for both `install` and `policies`. You can learn more [here](https://mdxjs.com/).

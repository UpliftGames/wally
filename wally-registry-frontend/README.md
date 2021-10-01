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

All files should be formatted with Prettier before committing. You can either (pick one):

- Install the Prettier editor extension, and turn on the "format file on save" setting in your editor
- Run the `npm run format` command before committing any files, which will format all changed files in-place.

## Recommended Editor Extensions

These extensions make working on this project easier and help prevent mistakes. Links for VS Code extensions are provided.

- [Styled Components](https://marketplace.visualstudio.com/items?itemName=jpoissonnier.vscode-styled-components) is important for syntax highlighting and IntelliSense for the Styled Components CSS.
- [MDX](https://marketplace.visualstudio.com/items?itemName=silvenon.mdx) is important for proper syntax highlighting in MDX files.
- [Prettier](https://marketplace.visualstudio.com/items?itemName=esbenp.prettier-vscode)
- [ESLint](https://marketplace.visualstudio.com/items?itemName=dbaeumer.vscode-eslint)

## Structure

### ./src

The main source code for the website.

#### ./src/assets

Any images that are imported by the website should go here. Make sure you crush them before committing (see below)

### ./icons

This folder contains SVG icons that are built into an icon font using fantasticon. They are usable with the `<Icon />` component.

### ./static

This folder will be copied into the build and anything in it will be available on the public website.

## MDX

This project makes use of MDX files, which is Markdown mixed with JSX. MDX can be used for pages, and is used for both `install` and `policies`. You can learn more [here](https://mdxjs.com/).

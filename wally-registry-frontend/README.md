# Wally Registry Frontend

The Wally registry website serves as a place for Wally users to view information about the Wally package manager.

This is a [Next.js](https://nextjs.org) project bootstrapped with [`create-next-app`](https://nextjs.org/docs/app/api-reference/cli/create-next-app).

## Usage

First, run the development server:

```bash
npm run dev
# or
yarn dev
# or
pnpm dev
# or
bun dev
```

Open [http://localhost:3000](http://localhost:3000) with your browser to see the result.

To build the production server:

```bash
npm run build
```

All files should be formatted with Prettier before committing. You can either (pick one):

- Install the Prettier editor extension, and turn on the "format file on save" setting in your editor
- Run the `npm run lint` command before committing any files, which will format all changed files in-place.

## Recommended Editor Extensions

These extensions make working on this project easier and help prevent mistakes. Links for VS Code extensions are provided.

- [Styled Components](https://marketplace.visualstudio.com/items?itemName=jpoissonnier.vscode-styled-components) is important for syntax highlighting and IntelliSense for the Styled Components CSS.
- [MDX](https://marketplace.visualstudio.com/items?itemName=silvenon.mdx) is important for proper syntax highlighting in MDX files.
- [Prettier](https://marketplace.visualstudio.com/items?itemName=esbenp.prettier-vscode)
- [ESLint](https://marketplace.visualstudio.com/items?itemName=dbaeumer.vscode-eslint)

## Structure

### ./src

The main source code for the website.

#### ./public/assets

Any images that are imported by the website should go here. Make sure you crush them before committing (see below)

### ./icons

This folder contains SVG icons that are built into an icon font using fantasticon. They are usable with the `<Icon />` component.

### Fonts

This project uses [`next/font`](https://nextjs.org/docs/app/building-your-application/optimizing/fonts) to automatically optimize and load [Iosevka](https://typeof.net/Iosevka/).

## MDX

This project makes use of MDX files, which is Markdown mixed with JSX. MDX can be used for pages, and is used for both `install` and `policies`. You can learn more [here](https://mdxjs.com/).

## .env

In order to run this project, you must create a `.env` file with the following fields:

```
NEXT_PUBLIC_WALLY_API_URL=""
```

The public, base Wally API endpoint is `https://api.wally.run`.

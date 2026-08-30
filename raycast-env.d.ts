/// <reference types="@raycast/api">

/* 🚧 🚧 🚧
 * This file is auto-generated from the extension's manifest.
 * Do not modify manually. Instead, update the `package.json` file.
 * 🚧 🚧 🚧 */

/* eslint-disable @typescript-eslint/ban-types */

type ExtensionPreferences = {}

/** Preferences accessible in all the extension's commands */
declare type Preferences = ExtensionPreferences

declare namespace Preferences {
  /** Preferences accessible in the `index` command */
  export type Index = ExtensionPreferences & {
  /** Image width - Image width in pixels when transformed to HTML. */
  "imageWidth": string
}
}

declare namespace Arguments {
  /** Arguments passed to the `index` command */
  export type Index = {
  /** ![image name](https://myimage.com/myimage) */
  "markdownImage": string,
  /** 300 */
  "imageWidth": string
}
}


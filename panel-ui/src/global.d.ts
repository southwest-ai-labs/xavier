/// <reference types="vite/client" />

declare const __APP_VERSION__: string;

declare module "@swal/maloca-embed";

interface SwalMalocaPanelAttributes {
  /** Memory namespace (e.g. "swal/xavier/community") */
  namespace?: string;
  /** Authentication token for the maloca panel */
  token?: string;
  /** Additional CSS class names */
  class?: string;
  /** Inline style */
  style?: string;
  /** Slot children */
  children?: React.ReactNode;
}

declare namespace React {
  namespace JSX {
    interface IntrinsicElements {
      "swal-maloca-panel": SwalMalocaPanelAttributes &
        React.HTMLAttributes<HTMLElement>;
    }
  }
}

declare namespace JSX {
  interface IntrinsicElements {
    "swal-maloca-panel": SwalMalocaPanelAttributes &
      React.HTMLAttributes<HTMLElement>;
  }
}

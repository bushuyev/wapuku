const path = require('path');
const HtmlWebpackPlugin = require('html-webpack-plugin');
const WasmPackPlugin = require("@wasm-tool/wasm-pack-plugin");
const CopyWebpackPlugin = require('copy-webpack-plugin');

module.exports = {
 
  mode: "development",
  plugins: [
    new CopyWebpackPlugin([ { from: 'data', to: 'data' } ])

  ],
  experiments: {
    asyncWebAssembly: true
  },
  devServer: {
    static: {
      directory: path.join(__dirname, 'data')
    },
    compress: false,
    port: 7777,
  },
  watchOptions: {
    aggregateTimeout: 1000,
    poll: 2000,
  }
};

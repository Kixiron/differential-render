const path = require("path");
const CopyWebpackPlugin = require("copy-webpack-plugin");
const WasmPackPlugin = require("@wasm-tool/wasm-pack-plugin");

const distPath = path.resolve(__dirname, "dist");

module.exports = (_env, argv) => {
    return {
        devServer: {
            contentBase: distPath,
            compress: argv.mode === "production",
            port: 8000,
        },
        entry: "./bootstrap.js",
        output: {
            path: distPath,
            filename: "dashboard.js",
            webassemblyModuleFilename: "dashboard.wasm",
        },
        plugins: [
            new CopyWebpackPlugin({
                patterns: [
                    { from: "./static", to: distPath },
                ],
            }),
            new WasmPackPlugin({
                crateDirectory: ".",
                extraArgs: "--no-typescript",
            }),
        ],
        module: {
            rules: [
                {
                    test: /\.s[ac]ss$/i,
                    use: [
                        "style-loader",
                        "css-loader",
                        {
                            loader: "sass-loader",
                            options: {
                                sassOptions: {
                                    outputStyle: "compressed",
                                },
                            },
                        },
                    ],
                },
            ],
        },
        watch: argv.mode !== "production",
    };
};

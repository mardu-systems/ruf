# OpenSSL LTS overlay

`openssl/` is the official vcpkg OpenSSL 3.5.4 port, git tree
`737382595bac2a92c7f8f54f120b53d2637af521`, with only its `version` and source
SHA-512 changed to OpenSSL 3.5.9 LTS. Its 19 original recipe/patch files are
retained. The vcpkg recipe is MIT licensed; see `LICENSE.txt`. OpenSSL's own
Apache-2.0 license is staged by the port as before.

The checksum is for the GitHub-generated `openssl-3.5.9` tag archive used by
`vcpkg_from_github`, not the different official release tarball used by the macOS
recipe. All nine source patches were checked and applied to that archive.

`native/build.ts` passes this overlay explicitly. Existing x64/ARM64 static-md
triplets, MSVC runtime selection and certificate validation are unchanged. The
CMake probe, Rust adapter build and SBOM validation reject Windows SDKs outside
the maintained 3.5 LTS series or below this recipe's patch version, including
stale 3.6.5 SDKs restored locally. CI hashes the overlay for both SDK and OpenSSL
caches. Windows compilation and runtime qualification still require Windows;
patch application and simulated CMake discovery do not establish those results.

Remove the overlay when the pinned official registry offers the maintained LTS
patch, updating the matching cache inputs and version checks together.

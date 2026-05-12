# Install script for directory: C:/Users/joker/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/hdf5-metno-src-0.9.5/ext/hdf5/src

# Set the install prefix
if(NOT DEFINED CMAKE_INSTALL_PREFIX)
  set(CMAKE_INSTALL_PREFIX "D:/binpack_to_h5/binpack_to_h5/target/release/build/hdf5-metno-src-ed955ea9d7754134/out")
endif()
string(REGEX REPLACE "/$" "" CMAKE_INSTALL_PREFIX "${CMAKE_INSTALL_PREFIX}")

# Set the install configuration name.
if(NOT DEFINED CMAKE_INSTALL_CONFIG_NAME)
  if(BUILD_TYPE)
    string(REGEX REPLACE "^[^A-Za-z0-9_]+" ""
           CMAKE_INSTALL_CONFIG_NAME "${BUILD_TYPE}")
  else()
    set(CMAKE_INSTALL_CONFIG_NAME "Release")
  endif()
  message(STATUS "Install configuration: \"${CMAKE_INSTALL_CONFIG_NAME}\"")
endif()

# Set the component getting installed.
if(NOT CMAKE_INSTALL_COMPONENT)
  if(COMPONENT)
    message(STATUS "Install component: \"${COMPONENT}\"")
    set(CMAKE_INSTALL_COMPONENT "${COMPONENT}")
  else()
    set(CMAKE_INSTALL_COMPONENT)
  endif()
endif()

# Is this installation the result of a crosscompile?
if(NOT DEFINED CMAKE_CROSSCOMPILING)
  set(CMAKE_CROSSCOMPILING "FALSE")
endif()

if(NOT CMAKE_INSTALL_LOCAL_ONLY)
  # Include the install script for the subdirectory.
  include("D:/binpack_to_h5/binpack_to_h5/target/release/build/hdf5-metno-src-ed955ea9d7754134/out/build/src/H5FDsubfiling/cmake_install.cmake")
endif()

if(CMAKE_INSTALL_COMPONENT STREQUAL "headers" OR NOT CMAKE_INSTALL_COMPONENT)
  file(INSTALL DESTINATION "${CMAKE_INSTALL_PREFIX}/include" TYPE FILE FILES
    "C:/Users/joker/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/hdf5-metno-src-0.9.5/ext/hdf5/src/hdf5.h"
    "C:/Users/joker/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/hdf5-metno-src-0.9.5/ext/hdf5/src/H5api_adpt.h"
    "C:/Users/joker/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/hdf5-metno-src-0.9.5/ext/hdf5/src/H5encode.h"
    "C:/Users/joker/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/hdf5-metno-src-0.9.5/ext/hdf5/src/H5public.h"
    "C:/Users/joker/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/hdf5-metno-src-0.9.5/ext/hdf5/src/H5Apublic.h"
    "C:/Users/joker/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/hdf5-metno-src-0.9.5/ext/hdf5/src/H5ACpublic.h"
    "C:/Users/joker/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/hdf5-metno-src-0.9.5/ext/hdf5/src/H5Cpublic.h"
    "C:/Users/joker/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/hdf5-metno-src-0.9.5/ext/hdf5/src/H5Dpublic.h"
    "C:/Users/joker/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/hdf5-metno-src-0.9.5/ext/hdf5/src/H5Epubgen.h"
    "C:/Users/joker/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/hdf5-metno-src-0.9.5/ext/hdf5/src/H5Epublic.h"
    "C:/Users/joker/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/hdf5-metno-src-0.9.5/ext/hdf5/src/H5ESdevelop.h"
    "C:/Users/joker/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/hdf5-metno-src-0.9.5/ext/hdf5/src/H5ESpublic.h"
    "C:/Users/joker/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/hdf5-metno-src-0.9.5/ext/hdf5/src/H5Fpublic.h"
    "C:/Users/joker/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/hdf5-metno-src-0.9.5/ext/hdf5/src/H5FDcore.h"
    "C:/Users/joker/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/hdf5-metno-src-0.9.5/ext/hdf5/src/H5FDdevelop.h"
    "C:/Users/joker/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/hdf5-metno-src-0.9.5/ext/hdf5/src/H5FDdirect.h"
    "C:/Users/joker/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/hdf5-metno-src-0.9.5/ext/hdf5/src/H5FDfamily.h"
    "C:/Users/joker/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/hdf5-metno-src-0.9.5/ext/hdf5/src/H5FDhdfs.h"
    "C:/Users/joker/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/hdf5-metno-src-0.9.5/ext/hdf5/src/H5FDlog.h"
    "C:/Users/joker/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/hdf5-metno-src-0.9.5/ext/hdf5/src/H5FDmirror.h"
    "C:/Users/joker/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/hdf5-metno-src-0.9.5/ext/hdf5/src/H5FDmpi.h"
    "C:/Users/joker/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/hdf5-metno-src-0.9.5/ext/hdf5/src/H5FDmpio.h"
    "C:/Users/joker/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/hdf5-metno-src-0.9.5/ext/hdf5/src/H5FDmulti.h"
    "C:/Users/joker/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/hdf5-metno-src-0.9.5/ext/hdf5/src/H5FDonion.h"
    "C:/Users/joker/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/hdf5-metno-src-0.9.5/ext/hdf5/src/H5FDpublic.h"
    "C:/Users/joker/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/hdf5-metno-src-0.9.5/ext/hdf5/src/H5FDros3.h"
    "C:/Users/joker/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/hdf5-metno-src-0.9.5/ext/hdf5/src/H5FDs3comms.h"
    "C:/Users/joker/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/hdf5-metno-src-0.9.5/ext/hdf5/src/H5FDsec2.h"
    "C:/Users/joker/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/hdf5-metno-src-0.9.5/ext/hdf5/src/H5FDsplitter.h"
    "C:/Users/joker/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/hdf5-metno-src-0.9.5/ext/hdf5/src/H5FDstdio.h"
    "C:/Users/joker/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/hdf5-metno-src-0.9.5/ext/hdf5/src/H5FDwindows.h"
    "C:/Users/joker/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/hdf5-metno-src-0.9.5/ext/hdf5/src/H5FDsubfiling/H5FDsubfiling.h"
    "C:/Users/joker/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/hdf5-metno-src-0.9.5/ext/hdf5/src/H5FDsubfiling/H5FDioc.h"
    "C:/Users/joker/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/hdf5-metno-src-0.9.5/ext/hdf5/src/H5Gpublic.h"
    "C:/Users/joker/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/hdf5-metno-src-0.9.5/ext/hdf5/src/H5Idevelop.h"
    "C:/Users/joker/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/hdf5-metno-src-0.9.5/ext/hdf5/src/H5Ipublic.h"
    "C:/Users/joker/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/hdf5-metno-src-0.9.5/ext/hdf5/src/H5Ldevelop.h"
    "C:/Users/joker/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/hdf5-metno-src-0.9.5/ext/hdf5/src/H5Lpublic.h"
    "C:/Users/joker/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/hdf5-metno-src-0.9.5/ext/hdf5/src/H5Mpublic.h"
    "C:/Users/joker/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/hdf5-metno-src-0.9.5/ext/hdf5/src/H5MMpublic.h"
    "C:/Users/joker/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/hdf5-metno-src-0.9.5/ext/hdf5/src/H5Opublic.h"
    "C:/Users/joker/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/hdf5-metno-src-0.9.5/ext/hdf5/src/H5Ppublic.h"
    "C:/Users/joker/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/hdf5-metno-src-0.9.5/ext/hdf5/src/H5PLextern.h"
    "C:/Users/joker/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/hdf5-metno-src-0.9.5/ext/hdf5/src/H5PLpublic.h"
    "C:/Users/joker/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/hdf5-metno-src-0.9.5/ext/hdf5/src/H5Rpublic.h"
    "C:/Users/joker/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/hdf5-metno-src-0.9.5/ext/hdf5/src/H5Spublic.h"
    "C:/Users/joker/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/hdf5-metno-src-0.9.5/ext/hdf5/src/H5Tdevelop.h"
    "C:/Users/joker/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/hdf5-metno-src-0.9.5/ext/hdf5/src/H5Tpublic.h"
    "C:/Users/joker/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/hdf5-metno-src-0.9.5/ext/hdf5/src/H5TSdevelop.h"
    "C:/Users/joker/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/hdf5-metno-src-0.9.5/ext/hdf5/src/H5VLconnector.h"
    "C:/Users/joker/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/hdf5-metno-src-0.9.5/ext/hdf5/src/H5VLconnector_passthru.h"
    "C:/Users/joker/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/hdf5-metno-src-0.9.5/ext/hdf5/src/H5VLnative.h"
    "C:/Users/joker/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/hdf5-metno-src-0.9.5/ext/hdf5/src/H5VLpassthru.h"
    "C:/Users/joker/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/hdf5-metno-src-0.9.5/ext/hdf5/src/H5VLpublic.h"
    "C:/Users/joker/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/hdf5-metno-src-0.9.5/ext/hdf5/src/H5Zdevelop.h"
    "C:/Users/joker/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/hdf5-metno-src-0.9.5/ext/hdf5/src/H5Zpublic.h"
    "C:/Users/joker/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/hdf5-metno-src-0.9.5/ext/hdf5/src/H5Epubgen.h"
    "C:/Users/joker/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/hdf5-metno-src-0.9.5/ext/hdf5/src/H5version.h"
    "C:/Users/joker/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/hdf5-metno-src-0.9.5/ext/hdf5/src/H5overflow.h"
    "D:/binpack_to_h5/binpack_to_h5/target/release/build/hdf5-metno-src-ed955ea9d7754134/out/build/src/H5pubconf.h"
    )
endif()

if(CMAKE_INSTALL_COMPONENT STREQUAL "libraries" OR NOT CMAKE_INSTALL_COMPONENT)
  if(CMAKE_INSTALL_CONFIG_NAME MATCHES "^([Dd][Ee][Bb][Uu][Gg])$")
    file(INSTALL DESTINATION "${CMAKE_INSTALL_PREFIX}/lib" TYPE FILE OPTIONAL FILES "D:/binpack_to_h5/binpack_to_h5/target/release/build/hdf5-metno-src-ed955ea9d7754134/out/build/bin/Debug/.pdb")
  elseif(CMAKE_INSTALL_CONFIG_NAME MATCHES "^([Rr][Ee][Ll][Ww][Ii][Tt][Hh][Dd][Ee][Bb][Ii][Nn][Ff][Oo])$")
    file(INSTALL DESTINATION "${CMAKE_INSTALL_PREFIX}/lib" TYPE FILE OPTIONAL FILES "D:/binpack_to_h5/binpack_to_h5/target/release/build/hdf5-metno-src-ed955ea9d7754134/out/build/bin/RelWithDebInfo/.pdb")
  endif()
endif()

if(CMAKE_INSTALL_COMPONENT STREQUAL "libraries" OR NOT CMAKE_INSTALL_COMPONENT)
  if(CMAKE_INSTALL_CONFIG_NAME MATCHES "^([Dd][Ee][Bb][Uu][Gg])$")
    file(INSTALL DESTINATION "${CMAKE_INSTALL_PREFIX}/lib" TYPE STATIC_LIBRARY FILES "D:/binpack_to_h5/binpack_to_h5/target/release/build/hdf5-metno-src-ed955ea9d7754134/out/build/bin/Debug/libhdf5_D.lib")
  elseif(CMAKE_INSTALL_CONFIG_NAME MATCHES "^([Rr][Ee][Ll][Ee][Aa][Ss][Ee])$")
    file(INSTALL DESTINATION "${CMAKE_INSTALL_PREFIX}/lib" TYPE STATIC_LIBRARY FILES "D:/binpack_to_h5/binpack_to_h5/target/release/build/hdf5-metno-src-ed955ea9d7754134/out/build/bin/Release/libhdf5.lib")
  elseif(CMAKE_INSTALL_CONFIG_NAME MATCHES "^([Mm][Ii][Nn][Ss][Ii][Zz][Ee][Rr][Ee][Ll])$")
    file(INSTALL DESTINATION "${CMAKE_INSTALL_PREFIX}/lib" TYPE STATIC_LIBRARY FILES "D:/binpack_to_h5/binpack_to_h5/target/release/build/hdf5-metno-src-ed955ea9d7754134/out/build/bin/MinSizeRel/libhdf5.lib")
  elseif(CMAKE_INSTALL_CONFIG_NAME MATCHES "^([Rr][Ee][Ll][Ww][Ii][Tt][Hh][Dd][Ee][Bb][Ii][Nn][Ff][Oo])$")
    file(INSTALL DESTINATION "${CMAKE_INSTALL_PREFIX}/lib" TYPE STATIC_LIBRARY FILES "D:/binpack_to_h5/binpack_to_h5/target/release/build/hdf5-metno-src-ed955ea9d7754134/out/build/bin/RelWithDebInfo/libhdf5.lib")
  endif()
endif()

if(CMAKE_INSTALL_COMPONENT STREQUAL "libraries" OR NOT CMAKE_INSTALL_COMPONENT)
  file(INSTALL DESTINATION "${CMAKE_INSTALL_PREFIX}/lib/pkgconfig" TYPE FILE FILES "D:/binpack_to_h5/binpack_to_h5/target/release/build/hdf5-metno-src-ed955ea9d7754134/out/build/CMakeFiles/hdf5.pc")
endif()

string(REPLACE ";" "\n" CMAKE_INSTALL_MANIFEST_CONTENT
       "${CMAKE_INSTALL_MANIFEST_FILES}")
if(CMAKE_INSTALL_LOCAL_ONLY)
  file(WRITE "D:/binpack_to_h5/binpack_to_h5/target/release/build/hdf5-metno-src-ed955ea9d7754134/out/build/src/install_local_manifest.txt"
     "${CMAKE_INSTALL_MANIFEST_CONTENT}")
endif()

#!/bin/sh
#
#  Generates documentation from .NET assemblies.
#
#  This command requires mdoc 5.0.0 or later, which is available from NuGet:
#  https://www.nuget.org/packages/mdoc/
#  It might work with an older version of mdoc, though.
#

set -x
cd "$(dirname "$0")/.."

MONODOC_DIR=Derived/Monodoc
HTML_DIR=Derived/MonodocHtml

MONODOC_FLAGS="--delete --fno-assembly-versions"

for p in Ngs.Interop Ngs.Engine.Facade Ngs.Framework; do
    TARGET_FX=netcoreapp2.1

    if [ "$p" == "Ngs.Interop" ]; then
        TARGET_FX=netstandard2.0
    fi

    dotnet build "$p/$p.csproj"
    MONODOC_FLAGS="$MONODOC_FLAGS $p/bin/Debug/$TARGET_FX/$p.dll -i $p/bin/Debug/$TARGET_FX/$p.xml"
done

mdoc update --out $MONODOC_DIR $MONODOC_FLAGS || exit 1

mdoc export-html $MONODOC_DIR --template Utils/MonodocTemplate.xsl --out $HTML_DIR || exit 1

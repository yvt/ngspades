//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
using System.Collections.Generic;
using System.Linq;
using System.Text.RegularExpressions;
namespace Ngs.Interop.CodeGen {
    /**
     * <summary>
     * Converts a set of words to and back from a snake case identifier (e.g, <c>io_Permission</c>, <c>Html_element</c>).
     * </summary>
     */
    sealed class SnakeCaseConverter {
        public static string[] Split(string text) {
            return text.Split('_');
        }

        public static string Join(string[] parts) {
            return string.Join("_", parts);
        }
    }

    /**
     * <summary>
     * Converts a set of words to and back from a .NET-style camel case identifier (e.g, <c>ioPermission</c>, <c>htmlElement</c>).
     * </summary>
     */
    sealed class DotNetCamelCaseConverter {
        static readonly Regex firstWordRegex = new Regex("^[a-z0-9](?:[a-z0-9]+|[a-z])");
        static readonly Regex wordRegex = new Regex("[A-Z0-9](?:[a-z0-9]+|[A-Z])|.");

        public static string[] Split(string text) {
            var list = new List<string>();
            var first = firstWordRegex.Match(text);
            if (first.Success) {
                list.Add(first.Value);
                text = text.Substring(first.Value.Length);
            }
            var matches = wordRegex.Matches(text);
            foreach (Match match in matches) {
                list.Add(match.Value);
            }
            return list.ToArray();
        }

        public static string Join(string[] parts) {
            return string.Join("", parts.Select((s, i) =>
              i == 0 ? s.ToLowerInvariant() :
              s.Substring(0, 1).ToUpperInvariant() + s.Substring(1).ToLowerInvariant()));
        }
    }

    /**
     * <summary>
     * Converts a set of words to and back from a .NET-style pascal case identifier (e.g, <c>IOPermission</c>, <c>HtmlElement</c>).
     * </summary>
     */
    sealed class DotNetPascalCaseConverter {
        static readonly Regex wordRegex = new Regex("[A-Z0-9](?:[a-z0-9]+|[A-Z])|.");

        public static string[] Split(string text) {
            var matches = wordRegex.Matches(text);
            var list = new List<string>();
            foreach (Match match in matches) {
                list.Add(match.Value);
            }
            return list.ToArray();
        }

        public static string Join(string[] parts) {
            return string.Join("", parts.Select((s) => s.Substring(0, 1).ToUpperInvariant() + s.Substring(1).ToLowerInvariant()));
        }
    }
}
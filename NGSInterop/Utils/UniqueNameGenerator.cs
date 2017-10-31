//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
using System.Collections.Generic;

namespace Ngs.Interop.Utils
{
    sealed class UniqueNameGenerator
    {
        Dictionary<string, int> generatedNames = new Dictionary<string, int>();

        public UniqueNameGenerator()
        {
        }

        public string Uniquify(string template)
        {
            int lastIndex;
            if (generatedNames.TryGetValue(template, out lastIndex))
            {
                ++lastIndex;
                generatedNames[template] = lastIndex;

                return $"{template}<{lastIndex}>";
            }

            generatedNames[template] = 0;
            return template;
        }
    }
}

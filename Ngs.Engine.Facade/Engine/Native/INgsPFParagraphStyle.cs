//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
using System;
using System.Runtime.InteropServices;
using Ngs.Interop;
using Ngs.Utils;
using Ngs.Engine.Canvas.Text;

namespace Ngs.Engine.Native {
    /// <summary>
    /// A set of paragraph styles.
    /// </summary>
    [Guid("676e6bdd-741e-432c-9de4-77a730619dae")]
    public interface INgsPFParagraphStyle : IUnknown {
        float MinimumLineHeight { get; set; }
        float LineHeightFactor { get; set; }
        TextAlign TextAlign { get; set; }
        TextDirection TextDirection { get; set; }
        WordWrapMode WordWrapMode { get; set; }
        INgsPFCharStyle CharStyle { get; }
    }
}
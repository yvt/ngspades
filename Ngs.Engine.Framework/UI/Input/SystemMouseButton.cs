//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
using System;

namespace Ngs.UI.Input {
    /// <summary>
    /// Represents a system mouse button.
    /// </summary>
    public sealed class SystemMouseButton : MouseButton {
        readonly byte pfType;
        readonly string name;

        private SystemMouseButton(byte pfType) {
            this.pfType = pfType;

            if (Type is MouseButtonType type) {
                this.name = type.ToString();
            } else {
                this.name = $"Button {(int)pfType + 1}";
            }
        }

        static SystemMouseButton[] buttons;

        static SystemMouseButton() {
            buttons = new SystemMouseButton[256];

            for (int i = 0; i < 256; ++i) {
                buttons[i] = new SystemMouseButton((byte)i);
            }
        }

        internal static SystemMouseButton GetButtonFromPFValue(byte pfType) => buttons[pfType];

        /// <summary>
        /// Retrieves the <see cref="SystemMouseButton" /> instance representing the left mouse
        /// button.
        /// </summary>
        public static SystemMouseButton Left => buttons[0];

        /// <summary>
        /// Retrieves the <see cref="SystemMouseButton" /> instance representing the right mouse
        /// button.
        /// </summary>
        public static SystemMouseButton Right => buttons[1];

        /// <summary>
        /// Retrieves the <see cref="SystemMouseButton" /> instance representing the center mouse
        /// button.
        /// </summary>
        public static SystemMouseButton Center => buttons[2];

        /// <summary>
        /// Implements <see cref="MouseButton.Type" />.
        /// </summary>
        public override MouseButtonType? Type {
            get {
                switch (pfType) {
                    case 0: return MouseButtonType.Left;
                    case 1: return MouseButtonType.Right;
                    case 2: return MouseButtonType.Center;
                    default: return (MouseButtonType?)null;
                }
            }
        }

        /// <summary>
        /// Implements <see cref="MouseButton.Name" />.
        /// </summary>
        public override string Name => name;
    }
}
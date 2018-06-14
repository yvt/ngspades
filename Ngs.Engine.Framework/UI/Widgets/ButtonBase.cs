//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
using System;

namespace Ngs.Engine.UI.Widgets {
    /// <summary>
    /// A base class for command button-like widgets.
    /// </summary>
    /// <remarks>
    /// <para>This class provides and registers mouse and key event handlers in order to implement
    /// the basic behavior of command buttons.
    /// This class does not include any facilies to display visual elements by itself.
    /// Derived classes must implement their own rendering logics and/or delegate the rendering to
    /// subviews to display their contents.</para>
    /// <para>The current state of a button is indicated by the properties <see cref="IsPressed" />
    /// and <see cref="IsHovered" />. The <see cref="ButtonStateUpdated" /> event is raised whenever
    /// a change occurs to these properties.</para>
    /// </remarks>
    public abstract class ButtonBase : View {
        bool hot, pressed;

        /// <summary>
        /// Initializes a new instance of this class.
        /// </summary>
        public ButtonBase() {
            EnableMouseTracking = true;
        }

        /// <summary>
        /// Indicates whether a mouse pointer is inside this view.
        /// </summary>
        public bool IsHovered => hot;

        /// <summary>
        /// Indicates whether this view is currently being pressed down.
        /// </summary>
        public bool IsPressed => pressed;

        /// <summary>
        /// Occurs when the command button was activated.
        /// </summary>
        public event EventHandler Activated;

        /// <summary>
        /// Called when the command button was activated.
        /// </summary>
        protected virtual void OnActivated(EventArgs e) => this.Activated?.Invoke(this, e);

        /// <summary>
        /// Occurs when the button state (<see cref="IsHovered" /> and <see cref="IsPressed" />) was
        /// updated.
        /// </summary>
        public event EventHandler ButtonStateUpdated;

        /// <summary>
        /// Called when the button state (<see cref="IsHovered" /> and <see cref="IsPressed" />) was
        /// updated.
        /// </summary>
        protected virtual void OnButtonStateUpdated(EventArgs e) =>
            this.ButtonStateUpdated?.Invoke(this, e);

        /// <summary>
        /// Overrides <see cref="View.OnMouseDown(Input.MouseButtonEventArgs)" />
        /// </summary>
        protected override void OnMouseDown(Ngs.Engine.UI.Input.MouseButtonEventArgs e) {
            if (e.Button.Type == Ngs.Engine.UI.Input.MouseButtonType.Left) {
                pressed = true;
                hot = true;
                OnButtonStateUpdated(EventArgs.Empty);
            }
        }

        /// <summary>
        /// Overrides <see cref="View.OnMouseUp(Input.MouseButtonEventArgs)" />
        /// </summary>
        protected override void OnMouseUp(Ngs.Engine.UI.Input.MouseButtonEventArgs e) {
            if (e.Button.Type == Ngs.Engine.UI.Input.MouseButtonType.Left) {
                if (pressed) {
                    pressed = false;
                    OnButtonStateUpdated(EventArgs.Empty);
                    if (hot) {
                        OnActivated(EventArgs.Empty);
                    }
                }
            }
        }

        /// <summary>
        /// Overrides <see cref="View.OnMouseCancel(Input.MouseButtonEventArgs)" />
        /// </summary>
        protected override void OnMouseCancel(Ngs.Engine.UI.Input.MouseButtonEventArgs e) {
            if (e.Button.Type == Ngs.Engine.UI.Input.MouseButtonType.Left) {
                pressed = false;
                OnButtonStateUpdated(EventArgs.Empty);
            }
        }

        /// <summary>
        /// Overrides <see cref="View.OnMouseMove(Input.MouseEventArgs)" />
        /// </summary>
        protected override void OnMouseMove(Input.MouseEventArgs e) {
            bool newHot = MouseHitTestLocal(e.Position.GetPositionInView(this).Value); ;
            if (hot != newHot) {
                hot = newHot;
                OnButtonStateUpdated(EventArgs.Empty);
            }
        }

        /// <summary>
        /// Overrides <see cref="View.OnMouseEnter(EventArgs)" />
        /// </summary>
        protected internal override void OnMouseEnter(EventArgs e) {
            if (!hot) {
                hot = true;
                OnButtonStateUpdated(EventArgs.Empty);
            }
        }

        /// <summary>
        /// Overrides <see cref="View.OnMouseLeave(EventArgs)" />
        /// </summary>
        protected internal override void OnMouseLeave(EventArgs e) {
            if (hot) {
                hot = false;
                OnButtonStateUpdated(EventArgs.Empty);
            }
        }

        /// <summary>
        /// Overrides <see cref="View.OnKeyDown(Input.KeyEventArgs)" />
        /// </summary>
        protected internal override void OnKeyDown(Input.KeyEventArgs e) {
            if (e.VirtualKeyCode == Presentation.VirtualKeyCode.Space) {
                OnActivated(EventArgs.Empty);
            }
        }
    }
}

//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
using System;
using System.Diagnostics;
using System.Numerics;
using System.Security;
using Ngs.Utils;

namespace Ngs.UI {
    /// <summary>
    /// A functional and presentational unit of a graphical user interface.
    /// </summary>
    public class View {

        // TODO

        private Workspace workspace;

        /// <summary>
        /// Initializes a new instance of this class.
        /// </summary>
        [SecuritySafeCritical]
        public View() {
            this.workspace = Application.EnsureInstance().Workspace;
            this.layerEmitter = new LayerEmitter(this.workspace.EngineWorkspace.Context);
        }

        #region Presentational properties

        public Matrix4x4 Transform { get; set; } = Matrix4x4.Identity;
        public float Opacity { get; set; } = 1;

        #endregion

        #region Layouting

        /// <summary>
        /// Retrieves the inherent minimum size of this view.
        /// </summary>
        /// <returns>The inherent minimum size of this view.</returns>
        public virtual Vector2 MinimumSize { get; } = new Vector2(0, 0);

        /// <summary>
        /// Retrieves the inherent maximum size of this view.
        /// </summary>
        /// <returns>The inherent maximum size of this view.</returns>
        public virtual Vector2 MaximumSize { get; } = new Vector2(float.PositiveInfinity, float.PositiveInfinity);

        /// <summary>
        /// Retrieves the inherent preferred size of this view.
        /// </summary>
        /// <returns>The inherent preferred size of this view.</returns>
        public virtual Vector2? PreferredSize { get; }

        /// <summary>
        /// Retrieves the bounding rectangle of this view within its superview.
        /// </summary>
        /// <remarks>
        /// This property is computed and set by a layout object associated with its superview.
        /// </remarks>
        /// <returns>The bounding rectangle.</returns>
        public Box2 Bounds { get; internal set; }

        /// <summary>
        /// Retrieves the result of the measurement step of the layout algorithm on this view.
        /// </summary>
        /// <remarks>
        /// The value of this property is affected by the inherent layout properties as well as
        /// those of the associated layout (if any).
        /// </remarks>
        /// <returns>The measurement result.</returns>
        public Measurement Measurement { get; internal set; }

        /// <summary>
        /// The layout where this view is located.
        /// </summary>
        /// <remarks>
        /// This value must not be exposed to the application. Otherwise, the invariants of
        /// <see cref="WindowContentsLayout" /> might be violated.
        /// </remarks>
        internal Layout superviewLayout;

        /// <summary>
        /// The layout used to position subviews of this view.
        /// </summary>
        private Layout layout;

        /// <summary>
        /// Sets or retrieves the layout object used to position the subviews of this view.
        /// </summary>
        /// <returns>The layout object or <c>null</c>.</returns>
        protected Layout Layout {
            get => this.layout;
            set {
                if (this.layout == value) {
                    return;
                }

                if (this.layout != null) {
                    this.layout.view = null;
                    this.layout = null;
                }
                if (value.view != null) {
                    throw new InvalidOperationException("The specified layout is already associated with a view.");
                }
                value.view = this;
                this.layout = value;

                SetNeedsMeasure();
                SetNeedsArrange();
            }
        }

        /// <summary>
        /// Retrieves the superview of this view.
        /// </summary>
        /// <returns>The superview or <c>null</c>.</returns>
        public View Superview {
            get => this.superviewLayout?.View;
        }

        /// <summary>
        /// Retrieves the window where this view is located.
        /// </summary>
        /// <returns>The window where this view is located.</returns>
        internal Window Window {
            get {
                if (this.superviewLayout is WindowContentsLayout winLayout) {
                    return winLayout.Window;
                } else {
                    return this.Superview?.Window;
                }
            }
        }

        /// <summary>
        /// Marks that some of the inherent layout properties might have changed.
        /// </summary>
        protected void InvalidateInherentLayoutProps() {
            SetNeedsMeasure();
        }

        internal virtual void BeforeLayout() {
            // Crude implementation for prototyping
            if (this.layout != null) {
                foreach (var child in this.layout.Subviews) {
                    child.BeforeLayout();
                }
            }
        }

        internal void SetNeedsMeasure() {
            // Crude implementation for prototyping
            // TODO: Introduce update flags
            this.workspace.SetNeedsUpdate();
        }

        internal void SetNeedsArrange() {
            // Crude implementation for prototyping
            // TODO: Introduce update flags
            this.workspace.SetNeedsUpdate();
        }

        /// <summary>
        /// Performs the measurement step of the layouting algorithm.
        /// </summary>
        [DebuggerNonUserCode]
        internal void Measure() {
            var measurement = Measurement.Default;
            if (this.layout is Layout layout) {
                measurement = layout.Measure();
            }

            try {
                var min = this.MinimumSize;
                var max = this.MaximumSize;

                measurement.MinimumSize = Vector2.Max(measurement.MinimumSize, min);
                measurement.MaximumSize = Vector2.Min(measurement.MaximumSize, max);

                if (this.PreferredSize is Vector2 x) {
                    measurement.PreferredSize = x;
                }

                measurement.PreferredSize = Vector2.Clamp(measurement.PreferredSize,
                    measurement.MinimumSize, measurement.MaximumSize);
            } catch (Exception e) {
                this.workspace.OnUnhandledException(e);
            }

            this.Measurement = measurement;
        }

        /// <summary>
        /// Performs the arranging step of the layouting algorithm.
        /// </summary>
        [DebuggerNonUserCode]
        internal void Arrange() {
            if (this.layout is Layout layout) {
                try {
                    layout.ArrangeInternal();
                } catch (Exception e) {
                    this.workspace.OnUnhandledException(e);
                }
                foreach (var child in layout.Subviews) {
                    child.Arrange();
                }
            }
        }

        #endregion

        #region Rendering

        readonly LayerEmitter layerEmitter;

        /// <summary>
        /// Marks that this view must be rendered again before being displayed in the next frame.
        /// </summary>
        protected void SetNeedsRender() {
            // Crude implementation for prototyping
            // TODO: Introduce update flags
            this.workspace.SetNeedsUpdate();
        }

        /// <summary>
        /// Provides methods used to describe the layer tree of a view.
        /// </summary>
        /// <remarks>
        /// <para>An implementation of <see cref="View.RenderContents" /> calls the methods provided by
        /// this type to describe the layer tree of the corresponding view. A layer tree can
        /// include the following types of nodes:</para>
        /// <list type="bullet">
        ///     <item><term>
        ///         <em>Layers</em>. They are defined by
        ///         <see cref="BeginLayer(LayerInfo)" />, <see cref="EmitLayer(LayerInfo)" /> or
        ///         their respective overloads with a custom key, with a
        ///         <see cref="LayerInfo" /> value supplied as its parameter.
        ///     </term></item>
        ///     <item><term>
        ///         <em>Groups</em>. They serve as a mean to group zero or more child nodes
        ///         to assist the reconciliation process.  They are defined by
        ///         of <see cref="BeginLayer(LayerInfo)" />, <see cref="EmitLayer(LayerInfo)" /> or
        ///         their respective overloads with a custom key, with a
        ///         <c>null</c> value supplied as its <c>layerInfo</c> parameter.
        ///     </term></item>
        ///     <item><term>
        ///         Each layer optionally can have a <em>layer mask group</em> defined by
        ///         <see cref="BeginMaskGroup" />.
        ///     </term></item>
        /// </list>
        /// <para>The framework constructs a layer tree based on the description. When a layer
        /// tree is generated for the second time, the framework tries to minimize the number of
        /// nodes updated through a process called <em>reconciliation</em>.
        /// This process works as follows: It takes two input layer tree, before the update and after
        /// the update. If their root nodes have the exact same type (the dynamic type of their
        /// layer infos also must match for layers), only the properties of the node that actually
        /// changed are updated. Otherwise, the node is created again from scratch.
        /// After that, the process proceeds to their child nodes. Two sets of child nodes from
        /// those layer trees are compared to figure out which nodes are common within the set.
        /// Instead of comparing their properties and guessing the best match (which is a
        /// computationally hard problem), a <em>key</em> associated with each child node is used as
        /// a proxy for establishing correspondence. There are two types of keys:
        /// </para>
        /// <list type="bullet">
        ///     <item><term>
        ///         <em>Custom keys</em> supplied via the <c>key</c> parameter of the methods that
        ///         define a layer/group.
        ///     </term></item>
        ///     <item><term>
        ///         <em>Ordinal keys</em> are generated when none are supplied.
        ///         They are derived using a counter that is resetted when a parent node is defined
        ///         and increases every time a child node is defined on it.
        ///         Ordinal keys perform slightly better than custom keys.
        ///     </term></item>
        /// </list>
        /// <para>After that, the reconciliation algorithm is applied recursively on every matched
        /// node pair.</para>
        /// <para>From a functional point of view, it "just works" even if you don't supply a key
        /// at all. However, you can optimize the rendering performance by understanding the
        /// reconciliation process and supplying keys appropriately. This is especially
        /// the case if your <see cref="View" /> has a dynamically-changing layer tree.
        /// For example, suppose a case where you need to display a certain child layer depending on
        /// a dynamic condition. If you simply skip a call to <see cref="EmitLayer(LayerInfo)" />
        /// for this layer, every ordinal key of child layer defined after that layer would be
        /// shifted by one, leading to a suboptimal reconciliation performance.
        /// This could have been avoided by calling <c>EmitLayer(null)</c> (which defines an empty
        /// group node) instead.
        /// </para>
        /// </remarks>
        public readonly ref struct RenderContext {
            readonly View view;

            internal RenderContext(View view) {
                this.view = view;
            }

            LayerEmitter LayerEmitter { get => view.layerEmitter; }

            /// <summary>
            /// Starts encoding a child layer or group.
            /// </summary>
            /// <remarks>
            /// <para>This method instructs the framework to create a layer as a child of the
            /// current node. After that, it starts a new nesting level to encode the children of
            /// the newly defined node.</para>
            /// <para>This overload uses a ordinal key for reconciliation.</para>
            /// <para>Calls to this method must be matched by the same number of calls to
            /// <see cref="EndLayer" />.</para>
            /// </remarks>
            /// <param name="layerInfo">The <see cref="LayerInfo"/> object that describes the
            /// the properties of the layer. <c>null</c> indicates a group.</param>
            public void BeginLayer(LayerInfo layerInfo) {
                LayerEmitter.BeginLayer(null, layerInfo);
            }

            /// <summary>
            /// Starts encoding a child layer or group.
            /// </summary>
            /// <remarks>
            /// <para>This method instructs the framework to create a layer as a child of the
            /// current node. After that, it starts a new nesting level to encode the children of
            /// the newly defined node.</para>
            /// <para>This overload uses a supplied key for reconciliation.</para>
            /// <para>Calls to this method must be matched by the same number of calls to
            /// <see cref="EndLayer" />.</para>
            /// </remarks>
            /// <param name="key">The key used for the reconciliation process.</param>
            /// <param name="layerInfo">The <see cref="LayerInfo"/> object that describes the
            /// the properties of the layer. <c>null</c> indicates a group.</param>
            /// <exception name="ArgumentException">A child node with the specified key already
            /// exists in the current node.</exception>
            public void BeginLayer(object key, LayerInfo layerInfo) {
                LayerEmitter.BeginLayer(key, layerInfo);
            }

            /// <summary>
            /// Ends the encoding of a layer or group that was started by <see cref="M:BeginLayer" />.
            /// </summary>
            public void EndLayer() {
                LayerEmitter.End();
            }

            /// <summary>
            /// Starts encoding a mask for the current layer.
            /// </summary>
            /// <remarks>
            /// <para>You can encode the mask contents after calling this method.
            /// A call to this method must be matched by a call to <see cref="EndMaskGroup" />.</para>
            /// </remarks>
            /// <exception name="InvalidOperationException">(1) The current node is not a layer.
            /// That is, the current nesting level was not started by a call to one of the overloads
            /// of <see cref="BeginLayer" /> with a <see cref="LayerInfo" /> value supplied as
            /// <c>layerInfo</c>.
            /// (2) A mask is already defined on the current node.
            /// </exception>
            /// <seealso cref="Ngs.Engine.Native.INgsPFLayer.Mask" />
            public void BeginMaskGroup() {
                LayerEmitter.BeginMaskGroup();
            }

            /// <summary>
            /// Ends the encoding a mask that was started by <see cref="BeginMaskGroup" />.
            /// </summary>
            public void EndMaskGroup() {
                LayerEmitter.End();
            }

            /// <summary>
            /// Encodes a child layer or placeholder (empty group).
            /// </summary>
            /// <remarks>
            /// <para>This method instructs the framework to create a layer as a child of the
            /// current node. However, it does not start a new nesting level. Use the
            /// <see cref="BeginLayer(LayerInfo)" /> method if you need to define child layers as
            /// well.</para>
            /// <para>This overload uses a ordinal key for reconciliation.</para>
            /// </remarks>
            /// <param name="layerInfo">The <see cref="LayerInfo"/> object that describes the
            /// the properties of the layer. <c>null</c> indicates a group.</param>
            public void EmitLayer(LayerInfo layerInfo) {
                BeginLayer(layerInfo);
                EndLayer();
            }

            /// <summary>
            /// Encodes a child layer or placeholder (empty group).
            /// </summary>
            /// <remarks>
            /// <para>This method instructs the framework to create a layer as a child of the
            /// current node. However, it does not start a new nesting level. Use the
            /// <see cref="BeginLayer(object,LayerInfo)" /> method if you need to define child
            /// layers as well.</para>
            /// <para>This overload uses a supplied key for reconciliation.</para>
            /// </remarks>
            /// <param name="key">The key used for the reconciliation process.</param>
            /// <param name="layerInfo">The <see cref="LayerInfo"/> object that describes the
            /// the properties of the layer. <c>null</c> indicates a group.</param>
            /// <exception name="ArgumentException">A child node with the specified key already
            /// exists in the current node.</exception>
            public void EmitLayer(object key, LayerInfo layerInfo) {
                BeginLayer(key, layerInfo);
                EndLayer();
            }

            /// <summary>
            /// Encodes a layer subtree of a subview.
            /// </summary>
            /// <param name="view"></param>
            /// <exception name="InvalidOperationException">The specified view is not a subview of
            /// the current view.</exception>
            public void EmitSubview(View view) {
                if (view.Superview != view) {
                    throw new InvalidOperationException("The view is not a subview of the current view.");
                }
                // TODO: Re-render only if necessary
                view.Render();
                LayerEmitter.EmitInstantiatedLayer(view.MainPFLayer);
            }

            /// <summary>
            /// Generates layers for all subviews at the current position.
            /// </summary>
            /// <remarks>
            /// <para>This is a utility method that automatically calls <see cref="EmitSubview" />
            /// on every subview of the current view. It does nothing if the current view does not
            /// have an associated layout.</para>
            /// <para></para>
            /// </remarks>
            public void EmitSubviews() {
                if (view.Layout is Layout layout) {
                    foreach (var view in layout.Subviews) {
                        EmitSubview(view);
                    }
                }
            }
        }

        /// <summary>
        /// Generates the (presentation) layer subtree for this view.
        /// </summary>
        /// <remarks>
        /// <para>The framework calls this method to generate the layer subtree for this view.
        /// The default implementation calls <see cref="RenderContext.EmitSubviews" />. The
        /// application can override this method to provide custom contents.</para>
        /// <para>The implementation describes the layer tree by calling methods on a supplied
        /// <see cref="RenderContext"/>.</para>
        /// <para>The local coordinate space has its origin point at this view's top-left corner.
        /// That is, the view's bounding rectangle (<see cref="Bounds" />) corresponds to the
        /// rectangle <c>(0, 0)-(Bounds.Width, Bounds.Height)</c>.</para>
        /// </remarks>
        /// <param name="context">The <see cref="RenderContext"/> object used to describe the layer
        /// tree.</param>
        protected virtual void RenderContents(RenderContext context) {
            context.EmitSubviews();
        }

        [DebuggerNonUserCode]
        internal void Render() {
            var emitter = this.layerEmitter;
            emitter.BeginUpdate();

            // Emit the main layer (the root of the layer subtree emitted by this view).
            emitter.BeginLayer(null, new LayerInfo()
            {
                // TODO: more props
                Bounds = this.Bounds,
                Opacity = this.Opacity,
            });

            // Emit the contents
            try {
                this.RenderContents(new RenderContext(this));
            } catch (Exception e) {
                this.workspace.OnUnhandledException(e);
            }

            emitter.End();

            emitter.EndUpdate();
        }

        internal Ngs.Interop.IUnknown MainPFLayer { get => this.layerEmitter.Root; }

        #endregion

        #region Focus management

        private bool acceptsFocus;
        private bool deniesFocus;

        /// <summary>
        /// Sets or retrieves a flag indicating whether this view can have a focus.
        /// </summary>
        /// <remarks>
        /// When this property is set to <c>false</c>, <see cref="Focused" /> is forced to be
        /// <c>false</c>.
        /// </remarks>
        /// <returns><c>true</c> if this view can have a focus; otherwise, <c>false</c>.</returns>
        protected internal bool AcceptsFocus {
            get => acceptsFocus;
            set {
                if (acceptsFocus == value) {
                    return;
                }
                acceptsFocus = value;
                if (!value) {
                    Focused = false;
                }
            }
        }

        /// <summary>
        /// Sets or retrieves a flag indicating whether this view and its descendants can have a focus.
        /// </summary>
        /// <remarks>
        /// When this property is set to <c>true</c>, <see cref="HasFocus" /> is forced to be
        /// <c>false</c>.
        /// </remarks>
        /// <returns><c>true</c> if this and its descendants cannot have a focus; otherwise,
        /// <c>false</c>.</returns>
        protected internal bool DeniesFocus {
            get => deniesFocus;
            set {
                if (deniesFocus == value) {
                    return;
                }
                deniesFocus = value;
                if (value) {
                    HasFocus = false;
                }
            }
        }

        bool HasAncestorDenyingFocus {
            get {
                for (var view = this; view != null; view = view.Superview) {
                    if (view.DeniesFocus) {
                        return true;
                    }
                }
                return false;
            }
        }

        internal bool CanGetFocus {
            get => AcceptsFocus && !HasAncestorDenyingFocus;
        }

        /// <summary>
        /// Sets or retrieves a flag indicating whether this view has a focus.
        /// </summary>
        /// <remarks>
        /// Setting this property to <c>true</c> on a view that is not allowed to own a focus
        /// (because its <see cref="AcceptsFocus" /> property is set to <c>false</c>,
        /// its descendant's <see cref="DeniesFocus" /> property is set to <c>true</c>, or the view
        /// is not located in a window) has no effects.
        /// </remarks>
        /// <returns><c>true</c> if this view is focused; otherwise, <c>false</c>.</returns>
        protected bool Focused {
            get => Window?.FocusedView == this;
            set {
                if (value == Focused) {
                    return;
                }

                var window = Window;
                if (window == null) {
                    // The view is not located in a window - noop
                    return;
                }

                if (value) {
                    if (!CanGetFocus) {
                        // This view can't get a focus
                        return;
                    }
                    window.FocusedView = this;
                } else {
                    window.FocusedView = null;
                }
            }
        }

        /// <summary>
        /// Sets or retrieves a flag indicating whether one of this view and its descendants has
        /// a focus.
        /// </summary>
        /// <returns><c>true</c> if one of this view and its descendants is focused; otherwise,
        /// <c>false</c>.</returns>
        public bool HasFocus {
            get {
                for (var view = Window?.FocusedView; view != null; view = view.Superview) {
                    if (view == this) {
                        return true;
                    }
                }
                return false;
            }
            set {
                if (value == HasFocus) {
                    return;
                }

                if (value) {
                    if (DefaultFocusView is View view) {
                        view.Focused = true;
                    }
                } else {
                    Window.FocusedView.Focused = false;
                }
            }
        }

        /// <summary>
        /// Retrieves the default (sub)view to acquire a focus when <see cref="HasFocus" /> was
        /// set to <c>true</c>.
        /// </summary>
        /// <remarks>
        /// The default implementation traverses the subview tree in the pre-order and returns
        /// the first encountered view that accepts a focus.
        /// </remarks>
        /// <returns>The view to acquire a focus. Must be <c>this</c>, its descendant, or
        /// <c>null</c>.</returns>
        protected virtual View DefaultFocusView {
            get {
                if (CanGetFocus) {
                    return this;
                }
                if (DeniesFocus) {
                    return null;
                }
                if (Layout is Layout layout) {
                    foreach (var subview in Layout.Subviews) {
                        if (subview.DefaultFocusView is View view) {
                            return view;
                        }
                    }
                }
                return null;
            }
        }

        /// <summary>
        /// Called when the view receives focus.
        /// </summary>
        /// <param name="e">The event data.</param>
        protected internal virtual void OnGotFocus(EventArgs e) { }

        /// <summary>
        /// Called when the view or its descendants receive focus.
        /// </summary>
        /// <param name="e">The event data.</param>
        protected internal virtual void OnEnter(EventArgs e) { }

        /// <summary>
        /// Called when the view loses focus.
        /// </summary>
        /// <param name="e">The event data.</param>
        protected internal virtual void OnLostFocus(EventArgs e) { }

        /// <summary>
        /// Called when the view or its descendants lose focus.
        /// </summary>
        /// <param name="e">The event data.</param>
        protected internal virtual void OnLeave(EventArgs e) { }

        #endregion

        #region Mouse/touch event handling

        protected bool EnableMouseTracking { get; set; }

        protected bool DeniesMouseInput { get; set; }

        protected virtual void OnMouseMove(MouseEventArgs e) { }
        protected virtual void OnMouseDown(MouseButtonEventArgs e) { }
        protected virtual void OnMouseUp(MouseButtonEventArgs e) { }
        protected virtual void OnMouseCancel(MouseButtonEventArgs e) { }

        #endregion
    }
}

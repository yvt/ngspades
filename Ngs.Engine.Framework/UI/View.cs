//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
using System;
using System.ComponentModel;
using System.Collections.Generic;
using System.Diagnostics;
using System.Numerics;
using System.Security;
using Ngs.Engine;
using Ngs.Engine.UI.Utils;

namespace Ngs.Engine.UI {
    /// <summary>
    /// A functional and presentational unit of a graphical user interface.
    /// </summary>
    public class View : ISynchronizeInvoke {
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

        // TODO: Add `LayoutTransform`

        private Matrix4 renderTransform = Matrix4.Identity;
        private bool renderTransformIsIdentity = true;
        private Vector2 renderTransformOrigin = new Vector2(0.5f, 0.5f);

        /// <summary>
        /// Sets or retrieves the transformation matrix applied to the layout box of this view.
        /// </summary>
        /// <returns>The transformation matrix.</returns>
        /// <seealso cref="RenderTransformOrigin" />
        public Matrix4 RenderTransform {
            get => renderTransform;
            set {
                if (value == renderTransform) {
                    return;
                }
                renderTransform = value;
                renderTransformIsIdentity = value == Matrix4.Identity;
                SetNeedsRender();
            }
        }

        /// <summary>
        /// Sets or retrieves the origin point used to apply <see cref="RenderTransform" />.
        /// </summary>
        /// <remarks>
        /// The default value is <c>(0.5, 0.5)</c> (center of the laout box).
        /// </remarks>
        /// <returns>The origin point, specified in relative to the layout box of this view.
        /// For example, <c>(0, 1)</c> and <c>(0.5, 0.5)</c> indicate the bottom-left corner and
        /// center of the box, respectively.</returns>
        public Vector2 RenderTransformOrigin {
            get => renderTransformOrigin;
            set {
                if (value == renderTransformOrigin) {
                    return;
                }
                renderTransformOrigin = value;
                SetNeedsRender();
            }
        }

        private float opacity = 1;

        /// <summary>
        /// Sets or retrieves the opacity of this view.
        /// </summary>
        /// <returns>The opacity specified in range <c>[0, 1]</c>.</returns>
        public float Opacity {
            get => opacity;
            set {
                if (value == opacity) {
                    return;
                }
                opacity = Math.Clamp(value, 0, 1);
                SetNeedsRender();
            }
        }
        #endregion

        #region Coordinate space

        /// <summary>
        /// Retrieves the transformation matrix that can be used to transform points from this
        /// view's local coordinate space to that of the superview.
        /// </summary>
        /// <remarks>
        /// The calculation of this property depends on <see cref="Bounds" />. This means that the
        /// returned value is not valid before layouting is done.
        /// </remarks>
        /// <returns>The transformation matrix.</returns>
        public Matrix4 LocalTransform {
            get {
                Matrix4 m = MainPFLayerTransform;
                // TODO: Handle `FlattenContents`
                //       m = m * Matrix4.CreateScale(1, 1, 0);
                return m;
            }
        }

        Matrix4 MainPFLayerTransform {
            get {
                Matrix4 m;
                if (renderTransformIsIdentity) {
                    m = Matrix4.CreateTranslation(Bounds.Min.Extend(0));
                } else {
                    Vector2 origin = renderTransformOrigin * Bounds.Size; // (wha...!?)
                    m =
                        Matrix4.CreateTranslation((Bounds.Min + origin).Extend(0)) *
                        renderTransform *
                        Matrix4.CreateTranslation((-origin).Extend(0));
                }
                return m;
            }
        }

        /// <summary>
        /// Calculates the transformation matrix that can be used to transform points from the
        /// view's local coordinate space to the containing window's client coordinate space.
        /// </summary>
        /// <remarks>
        /// The returned matrix projects all points on the plane <c>z = 0</c> (because window is 2D).
        /// </remarks>
        /// <returns>The calculated transformation matrix. <c>null</c> if this view does not
        /// belong to a window.</returns>
        public Matrix4? GetLocalToWindowClientTransform() {
            if (this.Superview?.GetLocalToWindowClientTransform() is Matrix4 m) {
                return m * this.LocalTransform;
            } else if (IsWindowContentView) {
                return Matrix4.CreateScale(1, 1, 0) * this.LocalTransform;
            } else {
                return null;
            }
        }

        /// <summary>
        /// Calculates the transformation matrix that can be used to transform points from the
        /// containing window's client coordinate space to the view's local coordinate space.
        /// </summary>
        /// <remarks>
        /// <para>The returned transformation matrix disregards the Z component of the input
        /// (because window is 2D) and projects all points on the plane <c>z = 0</c>.</para>
        /// <para>This method returns an incorrect value if the transform of the opposite direction
        /// is degenerate, i.e., more than one local points map to a single point on a window.</para>
        /// </remarks>
        /// <returns>The calculated transformation matrix. <c>null</c> if this view does not
        /// belong to a window.</returns>
        public Matrix3? GetWindowClientToLocalPlaneTransform() {
            if (this.GetLocalToWindowClientTransform() is Matrix4 m) {
                // Drop the Z component
                Matrix3 xywM = new Matrix3(
                    m.C1.X, m.C2.X, m.C4.X,
                    m.C1.Y, m.C2.Y, m.C4.Y,
                    m.C1.W, m.C2.W, m.C4.W
                );

                if (!Matrix3.Invert(xywM, out xywM)) {
                    // Singular matrix - returns a dummy value
                    return Matrix3.Identity;
                }

                return xywM;
            } else {
                return null;
            }
        }

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

        internal bool IsWindowContentView { get => this.superviewLayout is WindowContentsLayout; }

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
                foreach (var subview in layout.Subviews) {
                    subview.Measure();
                }
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
                if (view.Superview != this.view) {
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
                // TODO: Add `LayerFlags`?
                Opacity = this.Opacity,
                Bounds = new Box2(Vector2.Zero, Bounds.Size),
                Transform = MainPFLayerTransform,
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
        /// Occurs when the view or its descendants receive focus.
        /// </summary>
        public event EventHandler Enter;

        /// <summary>
        /// Occurs when the view or its descendants lose focus.
        /// </summary>
        public event EventHandler Leave;

        /// <summary>
        /// Called when the view receives focus.
        /// </summary>
        /// <param name="e">The event data.</param>
        protected internal virtual void OnGotFocus(EventArgs e) { }

        /// <summary>
        /// Called when the view or its descendants receive focus.
        /// </summary>
        /// <param name="e">The event data.</param>
        protected internal virtual void OnEnter(EventArgs e) {
            Enter?.Invoke(this, e);
        }

        /// <summary>
        /// Called when the view loses focus.
        /// </summary>
        /// <param name="e">The event data.</param>
        protected internal virtual void OnLostFocus(EventArgs e) { }

        /// <summary>
        /// Called when the view or its descendants lose focus.
        /// </summary>
        /// <param name="e">The event data.</param>
        protected internal virtual void OnLeave(EventArgs e) {
            Leave?.Invoke(this, e);
        }

        #endregion

        #region Mouse/touch event handling

        /// <summary>
        /// Sets or retrieves a flag indicating whether this view can receive mouse events.
        /// </summary>
        /// <returns><c>true</c> if this view can receive mouse events; otherwise,
        /// <c>false</c>.</returns>
        protected bool EnableMouseTracking { get; set; }

        /// <summary>
        /// Sets or retrieves a flag indicating whether this view and its descendants can receive mouse events.
        /// </summary>
        /// <returns><c>true</c> if this and its descendants cannot receive mouse events; otherwise,
        /// <c>false</c>.</returns>
        protected bool DeniesMouseInput { get; set; }

        /// <summary>
        /// Determines whether a specified point is within the tracking region of this view.
        /// </summary>
        /// <param name="point">A <see cref="Vector2" /> value representing a point, specified in
        /// the view's local coordinate.</param>
        /// <returns><c>true</c> if the point is within the tracking region; otherwise,
        /// <c>false</c>.</returns>
        public virtual bool MouseHitTestLocal(Vector2 point) =>
            point.X >= 0 && point.Y >= 0 && point.X < Bounds.Width && point.Y < Bounds.Height;

        /// <summary></summary>
        /// <param name="point">A point in the view's local coordinate.</param>
        /// <returns>The view or <c>null</c> if none was found.</returns>
        internal View MouseHitTest(Vector2 point) {
            if (DeniesMouseInput) {
                return null;
            }

            if (EnableMouseTracking) {
                // This is costly (O(n²) in worst case where n is the number of views)
                // but easy to maintain
                // FIXME: Do something about this?
                if (this.GetWindowClientToLocalPlaneTransform() is Matrix3 m) {
                    Vector2 projected = m.TransformPoint(point);
                    if (MouseHitTestLocal(projected)) {
                        return this;
                    }
                    // TODO: Handle `FlattenContents`
                }
            }

            View result = null;
            if (Layout is Layout layout) {
                foreach (var subview in layout.Subviews) {
                    result = subview.MouseHitTest(point) ?? result;
                }
            }
            return result;
        }

        /// <summary>
        /// Represents a mouse capture state.
        /// </summary>
        internal class MouseCapture : IDisposable {
            Input.Point lastPoint;
            List<Input.MouseButton> pressedButtons = new List<Input.MouseButton>();

            public View View { get; private set; }

            // I wish I could use `pub(super)` here
            internal MouseCapture(View view) {
                this.View = view;
            }

            void AddButton(Input.MouseButton button) {
                this.pressedButtons.Add(button);
            }

            void RemoveButton(Input.MouseButton button) {
                var i = this.pressedButtons.IndexOf(button);
                if (i < 0) {
                    throw new InvalidOperationException();
                }
                this.pressedButtons.SwapAndRemoveAt(i);
            }

            [DebuggerNonUserCode]
            public void MouseDown(Input.MouseButtonEventArgs e) {
                AddButton(e.Button);
                this.lastPoint = e.Position;
                try {
                    this.View.OnMouseDown(e);
                } catch (Exception ex) {
                    this.View.workspace.OnUnhandledException(ex);
                }
            }

            [DebuggerNonUserCode]
            public void MouseMove(Input.MouseEventArgs e) {
                try {
                    this.View.OnMouseMove(e);
                } catch (Exception ex) {
                    this.View.workspace.OnUnhandledException(ex);
                }
            }

            [DebuggerNonUserCode]
            public void MouseUp(Input.MouseButtonEventArgs e) {
                RemoveButton(e.Button);
                try {
                    this.View.OnMouseUp(e);
                } catch (Exception ex) {
                    this.View.workspace.OnUnhandledException(ex);
                }
            }

            [DebuggerNonUserCode]
            public void MouseCancel(Input.MouseButtonEventArgs e) {
                RemoveButton(e.Button);
                try {
                    this.View.OnMouseCancel(e);
                } catch (Exception ex) {
                    this.View.workspace.OnUnhandledException(ex);
                }
            }

            /// <summary>
            /// Releases the mouse capture. All mouse presses are cancelled.
            /// </summary>
            [DebuggerNonUserCode]
            public void Dispose() {
                // Cancel all mouse button inputs
                try {
                    foreach (var button in this.pressedButtons) {
                        this.View.OnMouseCancel(new Input.MouseButtonEventArgs(this.lastPoint, button));
                    }
                } catch (Exception ex) {
                    this.View.workspace.OnUnhandledException(ex);
                }

                this.View.currentMouseCapturingDevice = null;

                this.View = null;
                this.lastPoint = null;
                this.pressedButtons = null;
            }
        }

        Input.MouseDevice currentMouseCapturingDevice;

        /// <summary>
        /// Attempts to mouse-capture this view.
        /// </summary>
        /// <remarks>
        /// The caller must call <see cref="MouseCapture.Dispose" /> to release the mouse capture.
        /// </remarks>
        /// <param name="device">The mouse device attempting to capture this view.</param>
        /// <returns>A mouse capture state object, or <c>null</c> on failure, which happens if
        /// another mouse device already has a mouse capture on this view.</returns>
        internal MouseCapture AcquireMouseCapture(Input.MouseDevice device) {
            if (currentMouseCapturingDevice == null) {
                currentMouseCapturingDevice = device;
                return new MouseCapture(this);
            } else {
                return null;
            }
        }

        /// <summary>
        /// Called when the mouse pointer moves in this view.
        /// </summary>
        /// <param name="e">The event data.</param>
        protected virtual void OnMouseMove(Input.MouseEventArgs e) { }

        /// <summary>
        /// Called when a mouse button is pressed in this view.
        /// </summary>
        /// <param name="e">The event data.</param>
        protected virtual void OnMouseDown(Input.MouseButtonEventArgs e) { }

        /// <summary>
        /// Called when a mouse button is released in this view.
        /// </summary>
        /// <param name="e">The event data.</param>
        protected virtual void OnMouseUp(Input.MouseButtonEventArgs e) { }

        /// <summary>
        /// Called when a mouse button input was disrupted for an unknown reason.
        /// </summary>
        /// <param name="e">The event data.</param>
        protected virtual void OnMouseCancel(Input.MouseButtonEventArgs e) { }

        /// <summary>
        /// Called when the mouse pointer enters to this view.
        /// </summary>
        /// <param name="e">The event data.</param>
        protected internal virtual void OnMouseEnter(EventArgs e) { }

        /// <summary>
        /// Called when the mouse pointer leaves from this view.
        /// </summary>
        /// <param name="e">The event data.</param>
        protected internal virtual void OnMouseLeave(EventArgs e) { }

        #endregion

        #region ISynchronizeInvoke implementation
        /// <summary>
        /// Implemenets <see cref="ISynchronizeInvoke.InvokeRequired" />.
        /// </summary>
        public bool InvokeRequired => workspace.DispatchQueue.InvokeRequired;

        /// <summary>
        /// Implemenets <see cref="ISynchronizeInvoke.BeginInvoke(Delegate, object[])" />.
        /// </summary>
        public IAsyncResult BeginInvoke(Delegate method, object[] args) =>
            workspace.DispatchQueue.BeginInvoke(method, args);

        /// <summary>
        /// Implemenets <see cref="ISynchronizeInvoke.EndInvoke(IAsyncResult)" />.
        /// </summary>
        public object EndInvoke(IAsyncResult result) =>
            workspace.DispatchQueue.EndInvoke(result);

        /// <summary>
        /// Implements <see cref="ISynchronizeInvoke.Invoke(Delegate, object[])" />.
        /// </summary>
        public object Invoke(Delegate method, object[] args) =>
            workspace.DispatchQueue.Invoke(method, args);
        #endregion
    }
}

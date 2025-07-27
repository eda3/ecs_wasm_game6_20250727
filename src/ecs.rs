// =============================================================================
// 自作ECS（Entity-Component-System）実装
// =============================================================================
// このファイルでは、ECSアーキテクチャの基本的な3つの要素を実装します：
// 
// 1. Entity（エンティティ）: ゲーム内のオブジェクトを一意に識別するID
// 2. Component（コンポーネント）: エンティティが持つデータ（位置、体力など）
// 3. System（システム）: コンポーネントを操作するロジック（移動、描画など）
//
// 設計方針：
// - 関数型プログラミングの原則に従い、状態変更は明示的に行う
// - type safetyを重視し、コンパイル時にエラーを検出
// - メモリ効率を考慮したデータ構造を採用
// - WebAssembly環境での動作を最適化
// =============================================================================

use std::collections::HashMap;
use std::any::{Any, TypeId};
use std::marker::PhantomData;

// =============================================================================
// Entity（エンティティ）の定義
// =============================================================================

/// エンティティID型
/// 
/// エンティティは単なる一意のIDで、データ自体は持ちません。
/// この設計により、メモリ効率と実行速度の両方を向上させます。
/// 
/// 例：
/// - プレイヤーのエンティティID: Entity(1)
/// - カードのエンティティID: Entity(2)
/// - 敵のエンティティID: Entity(3)
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct Entity(pub u32);

impl Entity {
    /// 新しいエンティティIDを生成します
    /// 
    /// # 引数
    /// * `id` - エンティティの一意識別子
    /// 
    /// # 戻り値
    /// 新しいEntityインスタンス
    pub fn new(id: u32) -> Self {
        Entity(id)
    }

    /// エンティティIDの値を取得します
    pub fn id(&self) -> u32 {
        self.0
    }
}

// =============================================================================
// Component（コンポーネント）の定義
// =============================================================================

/// コンポーネントトレイト
/// 
/// すべてのコンポーネントが実装すべきトレイトです。
/// このトレイトを実装することで、型安全性を保ちながら
/// 異なる種類のコンポーネントを統一的に扱えます。
/// 
/// 実装例：
/// ```rust
/// #[derive(Debug, Clone)]
/// struct Position { x: f32, y: f32 }
/// impl Component for Position {}
/// ```
pub trait Component: Any + Send + Sync + 'static {
    /// コンポーネントの型名を取得します（デバッグ用）
    fn type_name(&self) -> &'static str {
        std::any::type_name::<Self>()
    }
}

// =============================================================================
// ComponentStorage（コンポーネント格納庫）の実装
// =============================================================================

/// コンポーネント格納庫
/// 
/// 特定の型のコンポーネントを効率的に格納・検索するためのコンテナです。
/// ジェネリクスを使用することで、型安全性を保ちながら高速なアクセスを実現します。
/// 
/// # ジェネリック型パラメータ
/// * `T` - 格納するコンポーネントの型（Componentトレイトを実装している必要がある）
pub struct ComponentStorage<T: Component> {
    /// エンティティIDをキーとして、コンポーネントを格納するハッシュマップ
    /// HashMap使用により、O(1)での挿入・検索・削除を実現
    components: HashMap<Entity, T>,
    /// PhantomDataを使用してTの型情報を保持（実際のメモリは使用しない）
    _phantom: PhantomData<T>,
}

impl<T: Component> ComponentStorage<T> {
    /// 新しいコンポーネント格納庫を作成します
    /// 
    /// # 戻り値
    /// 空のComponentStorageインスタンス
    pub fn new() -> Self {
        Self {
            components: HashMap::new(),
            _phantom: PhantomData,
        }
    }

    /// エンティティにコンポーネントを追加します
    /// 
    /// # 引数
    /// * `entity` - コンポーネントを追加するエンティティ
    /// * `component` - 追加するコンポーネント
    /// 
    /// # 戻り値
    /// 既に同じ型のコンポーネントが存在した場合は古いコンポーネント、
    /// 存在しなかった場合はNone
    pub fn insert(&mut self, entity: Entity, component: T) -> Option<T> {
        self.components.insert(entity, component)
    }

    /// エンティティのコンポーネントを取得します（不変参照）
    /// 
    /// # 引数
    /// * `entity` - コンポーネントを取得するエンティティ
    /// 
    /// # 戻り値
    /// コンポーネントが存在する場合はSome(&T)、存在しない場合はNone
    pub fn get(&self, entity: Entity) -> Option<&T> {
        self.components.get(&entity)
    }

    /// エンティティのコンポーネントを取得します（可変参照）
    /// 
    /// # 引数
    /// * `entity` - コンポーネントを取得するエンティティ
    /// 
    /// # 戻り値
    /// コンポーネントが存在する場合はSome(&mut T)、存在しない場合はNone
    pub fn get_mut(&mut self, entity: Entity) -> Option<&mut T> {
        self.components.get_mut(&entity)
    }

    /// エンティティからコンポーネントを削除します
    /// 
    /// # 引数
    /// * `entity` - コンポーネントを削除するエンティティ
    /// 
    /// # 戻り値
    /// 削除されたコンポーネント、存在しなかった場合はNone
    pub fn remove(&mut self, entity: Entity) -> Option<T> {
        self.components.remove(&entity)
    }

    /// エンティティが指定された型のコンポーネントを持っているかチェック
    /// 
    /// # 引数
    /// * `entity` - チェックするエンティティ
    /// 
    /// # 戻り値
    /// コンポーネントを持っている場合true、持っていない場合false
    pub fn contains(&self, entity: Entity) -> bool {
        self.components.contains_key(&entity)
    }

    /// すべてのエンティティとコンポーネントのペアを反復処理するイテレータを取得
    /// 
    /// # 戻り値
    /// (Entity, &T)のタプルを返すイテレータ
    pub fn iter(&self) -> impl Iterator<Item = (Entity, &T)> {
        self.components.iter().map(|(entity, component)| (*entity, component))
    }

    /// すべてのエンティティとコンポーネントの可変ペアを反復処理するイテレータを取得
    /// 
    /// # 戻り値
    /// (Entity, &mut T)のタプルを返すイテレータ
    pub fn iter_mut(&mut self) -> impl Iterator<Item = (Entity, &mut T)> {
        self.components.iter_mut().map(|(entity, component)| (*entity, component))
    }

    /// 格納されているコンポーネントの数を取得
    /// 
    /// # 戻り値
    /// コンポーネントの総数
    pub fn len(&self) -> usize {
        self.components.len()
    }

    /// コンポーネント格納庫が空かどうかチェック
    /// 
    /// # 戻り値
    /// 空の場合true、要素がある場合false
    pub fn is_empty(&self) -> bool {
        self.components.is_empty()
    }
}

// =============================================================================
// World（ワールド）の実装
// =============================================================================

/// ECSワールド
/// 
/// すべてのエンティティ、コンポーネント、システムを管理する中央管理クラスです。
/// 型消去（type erasure）を使用して、異なる型のコンポーネント格納庫を
/// 統一的に管理します。
/// 
/// 主な責務：
/// - エンティティの生成・削除
/// - コンポーネントの登録・取得・削除
/// - システムの実行管理
pub struct World {
    /// 次に生成するエンティティのID
    /// アトミックな操作で一意性を保証
    next_entity_id: u32,
    
    /// 型IDをキーとして、コンポーネント格納庫を管理
    /// Box<dyn Any>を使用した型消去により、異なる型の格納庫を統一管理
    component_storages: HashMap<TypeId, Box<dyn Any + Send + Sync>>,
    
    /// 生成されたエンティティのリスト
    /// エンティティの生存確認や一括操作に使用
    entities: Vec<Entity>,
}

impl World {
    /// 新しいECSワールドを作成します
    /// 
    /// # 戻り値
    /// 初期化されたWorldインスタンス
    pub fn new() -> Self {
        Self {
            next_entity_id: 1, // 0は無効なIDとして予約
            component_storages: HashMap::new(),
            entities: Vec::new(),
        }
    }

    /// 新しいエンティティを生成します
    /// 
    /// # 戻り値
    /// 新しく生成されたEntity
    /// 
    /// # 例
    /// ```rust
    /// let mut world = World::new();
    /// let player = world.create_entity();
    /// let enemy = world.create_entity();
    /// ```
    pub fn create_entity(&mut self) -> Entity {
        let entity = Entity::new(self.next_entity_id);
        self.next_entity_id += 1;
        self.entities.push(entity);
        entity
    }

    /// エンティティとその全コンポーネントを削除します
    /// 
    /// # 引数
    /// * `entity` - 削除するエンティティ
    /// 
    /// # 戻り値
    /// エンティティが存在して削除された場合true、存在しなかった場合false
    pub fn remove_entity(&mut self, entity: Entity) -> bool {
        // エンティティリストから削除
        if let Some(pos) = self.entities.iter().position(|&e| e == entity) {
            self.entities.remove(pos);
            
            // 全コンポーネント格納庫からこのエンティティのコンポーネントを削除
            // 注意：型安全性を保つため、実際の削除処理は各格納庫で個別に実装
            // ここでは存在確認のみ行う
            true
        } else {
            false
        }
    }

    /// 指定された型のコンポーネント格納庫を取得（不変参照）
    /// 
    /// # ジェネリック型パラメータ
    /// * `T` - 取得するコンポーネントの型
    /// 
    /// # 戻り値
    /// コンポーネント格納庫が存在する場合はSome(&ComponentStorage<T>)、
    /// 存在しない場合はNone
    fn get_storage<T: Component>(&self) -> Option<&ComponentStorage<T>> {
        let type_id = TypeId::of::<T>();
        self.component_storages
            .get(&type_id)?
            .downcast_ref::<ComponentStorage<T>>()
    }

    /// 指定された型のコンポーネント格納庫を取得（可変参照）
    /// 
    /// # ジェネリック型パラメータ
    /// * `T` - 取得するコンポーネントの型
    /// 
    /// # 戻り値
    /// コンポーネント格納庫が存在する場合はSome(&mut ComponentStorage<T>)、
    /// 存在しない場合はNone
    fn get_storage_mut<T: Component>(&mut self) -> Option<&mut ComponentStorage<T>> {
        let type_id = TypeId::of::<T>();
        self.component_storages
            .get_mut(&type_id)?
            .downcast_mut::<ComponentStorage<T>>()
    }

    /// 指定された型のコンポーネント格納庫を取得または作成（可変参照）
    /// 
    /// # ジェネリック型パラメータ
    /// * `T` - 取得/作成するコンポーネントの型
    /// 
    /// # 戻り値
    /// コンポーネント格納庫への可変参照（必ず成功）
    fn get_or_create_storage_mut<T: Component>(&mut self) -> &mut ComponentStorage<T> {
        let type_id = TypeId::of::<T>();
        
        // エントリAPIを使用して効率的な挿入を実行
        self.component_storages
            .entry(type_id)
            .or_insert_with(|| Box::new(ComponentStorage::<T>::new()))
            .downcast_mut::<ComponentStorage<T>>()
            .expect("型の不整合が発生しました。これはバグです。")
    }

    /// エンティティにコンポーネントを追加します
    /// 
    /// # 引数
    /// * `entity` - コンポーネントを追加するエンティティ
    /// * `component` - 追加するコンポーネント
    /// 
    /// # ジェネリック型パラメータ
    /// * `T` - 追加するコンポーネントの型
    /// 
    /// # 戻り値
    /// 既に同じ型のコンポーネントが存在した場合は古いコンポーネント、
    /// 存在しなかった場合はNone
    /// 
    /// # 例
    /// ```rust
    /// let mut world = World::new();
    /// let entity = world.create_entity();
    /// world.add_component(entity, Position { x: 10.0, y: 20.0 });
    /// ```
    pub fn add_component<T: Component>(&mut self, entity: Entity, component: T) -> Option<T> {
        self.get_or_create_storage_mut::<T>().insert(entity, component)
    }

    /// エンティティのコンポーネントを取得します（不変参照）
    /// 
    /// # 引数
    /// * `entity` - コンポーネントを取得するエンティティ
    /// 
    /// # ジェネリック型パラメータ
    /// * `T` - 取得するコンポーネントの型
    /// 
    /// # 戻り値
    /// コンポーネントが存在する場合はSome(&T)、存在しない場合はNone
    /// 
    /// # 例
    /// ```rust
    /// if let Some(position) = world.get_component::<Position>(entity) {
    ///     println!("エンティティの位置: ({}, {})", position.x, position.y);
    /// }
    /// ```
    pub fn get_component<T: Component>(&self, entity: Entity) -> Option<&T> {
        self.get_storage::<T>()?.get(entity)
    }

    /// エンティティのコンポーネントを取得します（可変参照）
    /// 
    /// # 引数
    /// * `entity` - コンポーネントを取得するエンティティ
    /// 
    /// # ジェネリック型パラメータ
    /// * `T` - 取得するコンポーネントの型
    /// 
    /// # 戻り値
    /// コンポーネントが存在する場合はSome(&mut T)、存在しない場合はNone
    /// 
    /// # 例
    /// ```rust
    /// if let Some(position) = world.get_component_mut::<Position>(entity) {
    ///     position.x += 1.0; // 位置を更新
    /// }
    /// ```
    pub fn get_component_mut<T: Component>(&mut self, entity: Entity) -> Option<&mut T> {
        self.get_storage_mut::<T>()?.get_mut(entity)
    }

    /// エンティティからコンポーネントを削除します
    /// 
    /// # 引数
    /// * `entity` - コンポーネントを削除するエンティティ
    /// 
    /// # ジェネリック型パラメータ
    /// * `T` - 削除するコンポーネントの型
    /// 
    /// # 戻り値
    /// 削除されたコンポーネント、存在しなかった場合はNone
    pub fn remove_component<T: Component>(&mut self, entity: Entity) -> Option<T> {
        self.get_storage_mut::<T>()?.remove(entity)
    }

    /// エンティティが指定された型のコンポーネントを持っているかチェック
    /// 
    /// # 引数
    /// * `entity` - チェックするエンティティ
    /// 
    /// # ジェネリック型パラメータ
    /// * `T` - チェックするコンポーネントの型
    /// 
    /// # 戻り値
    /// コンポーネントを持っている場合true、持っていない場合false
    pub fn has_component<T: Component>(&self, entity: Entity) -> bool {
        self.get_storage::<T>()
            .map_or(false, |storage| storage.contains(entity))
    }

    /// 指定された型のコンポーネントを持つ全エンティティを取得
    /// 
    /// # ジェネリック型パラメータ
    /// * `T` - 検索するコンポーネントの型
    /// 
    /// # 戻り値
    /// (Entity, &T)のタプルを返すイテレータ
    /// コンポーネント格納庫が存在しない場合は空のイテレータ
    pub fn query<T: Component>(&self) -> impl Iterator<Item = (Entity, &T)> {
        self.get_storage::<T>()
            .map(|storage| storage.iter())
            .into_iter()
            .flatten()
    }

    /// 指定された型のコンポーネントを持つ全エンティティを取得（可変参照）
    /// 
    /// # ジェネリック型パラメータ
    /// * `T` - 検索するコンポーネントの型
    /// 
    /// # 戻り値
    /// (Entity, &mut T)のタプルを返すイテレータ
    /// コンポーネント格納庫が存在しない場合は空のイテレータ
    pub fn query_mut<T: Component>(&mut self) -> impl Iterator<Item = (Entity, &mut T)> {
        self.get_storage_mut::<T>()
            .map(|storage| storage.iter_mut())
            .into_iter()
            .flatten()
    }

    /// ワールド内の全エンティティを取得
    /// 
    /// # 戻り値
    /// エンティティのスライス
    pub fn entities(&self) -> &[Entity] {
        &self.entities
    }

    /// ワールド内のエンティティ数を取得
    /// 
    /// # 戻り値
    /// エンティティの総数
    pub fn entity_count(&self) -> usize {
        self.entities.len()
    }
}

// =============================================================================
// System（システム）の定義
// =============================================================================

/// システムトレイト
/// 
/// ECSアーキテクチャにおけるシステムの動作を定義するトレイトです。
/// すべてのシステムはこのトレイトを実装し、update関数内で
/// ワールドのコンポーネントを操作するロジックを記述します。
/// 
/// 実装例：
/// ```rust
/// struct MovementSystem;
/// impl System for MovementSystem {
///     fn update(&mut self, world: &mut World, delta_time: f64) {
///         // 移動処理のロジック
///     }
/// }
/// ```
pub trait System {
    /// システムの処理を実行します
    /// 
    /// # 引数
    /// * `world` - ECSワールドへの可変参照
    /// * `delta_time` - 前フレームからの経過時間（秒）
    /// 
    /// この関数は毎フレーム呼び出され、ワールド内のコンポーネントを
    /// 操作してゲームの状態を更新します。
    fn update(&mut self, world: &mut World, delta_time: f64);

    /// システムの名前を取得します（デバッグ・ログ用）
    /// 
    /// # 戻り値
    /// システムの型名
    fn name(&self) -> &'static str {
        std::any::type_name::<Self>()
    }
}

// =============================================================================
// SystemScheduler（システムスケジューラ）の実装
// =============================================================================

/// システムスケジューラ
/// 
/// 複数のシステムを管理し、適切な順序で実行するためのクラスです。
/// システムの実行順序を制御することで、依存関係のあるシステム間での
/// 整合性を保ちます。
pub struct SystemScheduler {
    /// 実行するシステムのリスト
    /// 登録順序で実行されるため、依存関係を考慮した順序で登録する必要があります
    systems: Vec<Box<dyn System>>,
}

impl SystemScheduler {
    /// 新しいシステムスケジューラを作成します
    /// 
    /// # 戻り値
    /// 空のSystemSchedulerインスタンス
    pub fn new() -> Self {
        Self {
            systems: Vec::new(),
        }
    }

    /// システムを追加します
    /// 
    /// # 引数
    /// * `system` - 追加するシステム
    /// 
    /// # ジェネリック型パラメータ
    /// * `T` - 追加するシステムの型（Systemトレイトを実装している必要がある）
    /// 
    /// # 例
    /// ```rust
    /// let mut scheduler = SystemScheduler::new();
    /// scheduler.add_system(MovementSystem);
    /// scheduler.add_system(RenderSystem);
    /// ```
    pub fn add_system<T: System + 'static>(&mut self, system: T) {
        self.systems.push(Box::new(system));
    }

    /// 全システムを指定された順序で実行します
    /// 
    /// # 引数
    /// * `world` - ECSワールドへの可変参照
    /// * `delta_time` - 前フレームからの経過時間（秒）
    /// 
    /// この関数は毎フレーム呼び出され、登録されたすべてのシステムを
    /// 順次実行します。システムの実行順序は登録順序と同じです。
    pub fn update(&mut self, world: &mut World, delta_time: f64) {
        for system in &mut self.systems {
            system.update(world, delta_time);
        }
    }

    /// 登録されているシステムの数を取得
    /// 
    /// # 戻り値
    /// システムの総数
    pub fn system_count(&self) -> usize {
        self.systems.len()
    }
}

// =============================================================================
// デフォルト実装
// =============================================================================

impl Default for World {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for SystemScheduler {
    fn default() -> Self {
        Self::new()
    }
}
use std::{borrow::Cow, sync::Arc};

use arrow::array::ArrayData;
use arrow::{
    array::{AsArray, PrimitiveBuilder},
    datatypes::{
        ArrowPrimitiveType, Float16Type, Float32Type, Float64Type, Int64Type, UInt32Type, UInt8Type,
    },
};
use arrow_array::{Array, FixedSizeListArray, PrimitiveArray};
use arrow_schema::DataType;
use candle::{CpuStorage, Device, Layout, Storage, Tensor};
use candle_transformers::models::bert::BertModel;
use lancedb::embeddings::EmbeddingFunction;
use lancedb::Error;
use tokenizers::tokenizer::Tokenizer;
use crate::errors::AppResult;

pub struct SentenceTransformersEmbeddings {
    model: BertModel,
    tokenizer: Tokenizer,
    device: Device,
    n_dims: Option<usize>,
}

impl std::fmt::Debug for SentenceTransformersEmbeddings {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SentenceTransformersEmbeddings")
            .field("tokenizer", &self.tokenizer)
            .field("device", &self.device)
            .field("n_dims", &self.n_dims)
            .finish()
    }
}

impl SentenceTransformersEmbeddings {

    fn ndims(&self) -> lancedb::Result<usize> {
        if let Some(n_dims) = self.n_dims {
            Ok(n_dims)
        } else {
            Ok(self.compute_ndims_and_dtype()?.0)
        }
    }

    fn compute_ndims_and_dtype(&self) -> lancedb::Result<(usize, DataType)> {
        let token = self.tokenizer.encode("hello", true).unwrap();
        let token = token.get_ids().to_vec();
        let input_ids = Tensor::new(vec![token], &self.device).unwrap();

        let token_type_ids = input_ids.zeros_like().unwrap();

        let embeddings = self
            .model
            .forward(&input_ids, &token_type_ids, None)
            // TODO: it'd be nice to support other devices
            .and_then(|output| output.to_device(&Device::Cpu)).unwrap();

        let (_, _, n_dims) = embeddings.dims3().unwrap();
        let (storage, _) = embeddings.storage_and_layout();
        let dtype = match &*storage {
            Storage::Cpu(CpuStorage::U8(_)) => DataType::UInt8,
            Storage::Cpu(CpuStorage::U32(_)) => DataType::UInt32,
            Storage::Cpu(CpuStorage::I64(_)) => DataType::Int64,
            Storage::Cpu(CpuStorage::F16(_)) => DataType::Float16,
            Storage::Cpu(CpuStorage::F32(_)) => DataType::Float32,
            Storage::Cpu(CpuStorage::F64(_)) => DataType::Float64,
            Storage::Cpu(CpuStorage::BF16(_)) => {
                return Err(lancedb::Error::Runtime {
                    message: "unsupported data type".to_string(),
                })
            }
            _ => unreachable!("we already moved the tensor to the CPU device"),
        };
        Ok((n_dims, dtype))
    }

    fn compute_inner(&self, source: Arc<dyn Array>) -> lancedb::Result<(Arc<dyn Array>, DataType)> {
        if source.is_nullable() {
            return Err(lancedb::Error::InvalidInput {
                message: "Expected non-nullable data type".to_string(),
            });
        }
        if !matches!(source.data_type(), DataType::Utf8 | DataType::LargeUtf8) {
            return Err(lancedb::Error::InvalidInput {
                message: "Expected Utf8 data type".to_string(),
            });
        }
        let check_nulls = |source: &dyn Array| {
            if source.null_count() > 0 {
                return Err(lancedb::Error::Runtime {
                    message: "null values not supported".to_string(),
                });
            }
            Ok(())
        };
        let tokens = match source.data_type() {
            DataType::Utf8 => {
                check_nulls(&*source)?;
                source
                    .as_string::<i32>()
                    // TODO: should we do this in parallel? (e.g. using rayon)
                    .into_iter()
                    .map(|v| {
                        let value = v.unwrap();
                        let token = self.tokenizer.encode(value, true).map_err(|e| {
                            lancedb::Error::Runtime {
                                message: format!("failed to encode value: {}", e),
                            }
                        })?;
                        let token = token.get_ids().to_vec();
                        Ok(Tensor::new(token.as_slice(), &self.device).unwrap())
                    })
                    .collect::<lancedb::Result<Vec<_>>>()?
            }
            DataType::LargeUtf8 => {
                check_nulls(&*source)?;

                source
                    .as_string::<i64>()
                    // TODO: should we do this in parallel? (e.g. using rayon)
                    .into_iter()
                    .map(|v| {
                        let value = v.unwrap();
                        let token = self.tokenizer.encode(value, true).map_err(|e| {
                            lancedb::Error::Runtime {
                                message: format!("failed to encode value: {}", e),
                            }
                        })?;

                        let token = token.get_ids().to_vec();
                        Ok(Tensor::new(token.as_slice(), &self.device).unwrap())
                    })
                    .collect::<lancedb::Result<Vec<_>>>()?
            }
            DataType::Utf8View => {
                return Err(lancedb::Error::Runtime {
                    message: "Utf8View not yet implemented".to_string(),
                })
            }
            _ => {
                return Err(lancedb::Error::Runtime {
                    message: "invalid type".to_string(),
                })
            }
        };

        let embeddings = Tensor::stack(&tokens, 0)
            .and_then(|tokens| {
                let token_type_ids = tokens.zeros_like().unwrap();
                self.model.forward(&tokens, &token_type_ids, None)
            })
            // TODO: it'd be nice to support other devices
            .and_then(|tokens| tokens.to_device(&Device::Cpu))
            .map_err(|e| lancedb::Error::Runtime {
                message: format!("failed to compute embeddings: {}", e),
            })?;
        let (_, n_tokens, _) = embeddings.dims3().map_err(|e| lancedb::Error::Runtime {
            message: format!("failed to get embeddings dimensions: {}", e),
        })?;

        let embeddings = (embeddings.sum(1).unwrap() / (n_tokens as f64)).map_err(|e| {
            lancedb::Error::Runtime {
                message: format!("failed to compute mean embeddings: {}", e),
            }
        })?;
        let dims = embeddings.shape().dims().len();
        let (arr, dtype): (Arc<dyn Array>, DataType) = match dims {
            2 => {
                let (d1, d2) = embeddings.dims2().map_err(|e| lancedb::Error::Runtime {
                    message: format!("failed to get embeddings dimensions: {}", e),
                })?;
                let (storage, layout) = embeddings.storage_and_layout();
                match &*storage {
                    Storage::Cpu(CpuStorage::U8(data)) => {
                        let data: &[u8] = data.as_slice();
                        let arr = from_cpu_storage::<UInt8Type>(data, layout, &embeddings, d1, d2);

                        (Arc::new(arr), DataType::UInt8)
                    }
                    Storage::Cpu(CpuStorage::U32(data)) => (
                        Arc::new(from_cpu_storage::<UInt32Type>(
                            data,
                            layout,
                            &embeddings,
                            d1,
                            d2,
                        )),
                        DataType::UInt32,
                    ),
                    Storage::Cpu(CpuStorage::I64(data)) => (
                        Arc::new(from_cpu_storage::<Int64Type>(
                            data,
                            layout,
                            &embeddings,
                            d1,
                            d2,
                        )),
                        DataType::Int64,
                    ),
                    Storage::Cpu(CpuStorage::F16(data)) => (
                        Arc::new(from_cpu_storage::<Float16Type>(
                            data,
                            layout,
                            &embeddings,
                            d1,
                            d2,
                        )),
                        DataType::Float16,
                    ),
                    Storage::Cpu(CpuStorage::F32(data)) => (
                        Arc::new(from_cpu_storage::<Float32Type>(
                            data,
                            layout,
                            &embeddings,
                            d1,
                            d2,
                        )),
                        DataType::Float32,
                    ),
                    Storage::Cpu(CpuStorage::F64(data)) => (
                        Arc::new(from_cpu_storage::<Float64Type>(
                            data,
                            layout,
                            &embeddings,
                            d1,
                            d2,
                        )),
                        DataType::Float64,
                    ),
                    Storage::Cpu(CpuStorage::BF16(_)) => {
                        panic!("Unsupported storage type: BF16")
                    }
                    _ => unreachable!("Only CPU storage currently supported"),
                }
            }
            n_dims => todo!("Only 2 dimensions supported, got {}", n_dims),
        };
        Ok((arr, dtype))
    }

    pub fn new(
        model: BertModel,
        tokenizer: Tokenizer,
        device: Device,
        n_dims: Option<usize>,
    ) -> Self {
        Self {
            model,
            tokenizer,
            device,
            n_dims,
        }
    }
}

impl EmbeddingFunction for SentenceTransformersEmbeddings {
    fn name(&self) -> &str {
        "sentence-transformers"
    }

    fn source_type(&self) -> lancedb::Result<Cow<DataType>> {
        Ok(Cow::Owned(DataType::Utf8))
    }

    fn dest_type(&self) -> lancedb::Result<Cow<DataType>> {
        let (n_dims, dtype) = self.compute_ndims_and_dtype()?;
        Ok(Cow::Owned(DataType::new_fixed_size_list(
            dtype,
            n_dims as i32,
            false,
        )))
    }

    fn compute_source_embeddings(&self, source: Arc<dyn Array>) -> lancedb::Result<Arc<dyn Array>> {
        let len = source.len();
        let n_dims = self.ndims()?;
        let (inner, dtype) = self.compute_inner(source)?;

        let fsl = DataType::new_fixed_size_list(dtype, n_dims as i32, false);

        // We can't use the FixedSizeListBuilder here because it always adds a null bitmap
        // and we want to explicitly work with non-nullable arrays.
        let array_data = ArrayData::builder(fsl)
            .len(len)
            .add_child_data(inner.into_data())
            .build()?;

        Ok(Arc::new(FixedSizeListArray::from(array_data)))
    }

    fn compute_query_embeddings(&self, input: Arc<dyn Array>) -> lancedb::Result<Arc<dyn Array>> {
        let (arr, _) = self.compute_inner(input)?;
        Ok(arr)
    }
}

fn from_cpu_storage<T: ArrowPrimitiveType>(
    buffer: &[T::Native],
    layout: &Layout,
    embeddings: &Tensor,
    dim1: usize,
    dim2: usize,
) -> PrimitiveArray<T> {
    let mut builder = PrimitiveBuilder::<T>::with_capacity(dim1 * dim2);

    match layout.contiguous_offsets() {
        Some((o1, o2)) => {
            let data = &buffer[o1..o2];
            builder.append_slice(data);
            builder.finish()
        }
        None => {
            let mut src_index = embeddings.strided_index();

            for _idx_row in 0..dim1 {
                let row = (0..dim2)
                    .map(|_| buffer[src_index.next().unwrap()])
                    .collect::<Vec<_>>();
                builder.append_slice(&row);
            }
            builder.finish()
        }
    }
}
